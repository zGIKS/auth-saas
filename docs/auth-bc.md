# Authentication bounded context

## Visión general
`iam::authentication` agrupa el control de acceso: sign-in, logout, refresh, verificación de JWT y la federación con Google. Opera sobre la identidad confirmada del tenant actual (resuelta por `tenant_resolver`) usando:

- Repositorio Redis compartido (`RedisSessionRepository`) para JTIs, refresh tokens y listas negras.
- Servicio de tokens JWT (`JwtTokenService`) que genera `token` + `refresh_token`, valida firmas y expira automáticamente.
- Servicio de lockout (`AccountLockoutService`) y rate limiting (middleware global) para mitigar abuso.
- Consultas que revisan invalidaciones, revocaciones y coincidencia de JTI por sesión activa.

Todo se configura vía `AppState` (`SESSION_DURATION_SECONDS`, `REFRESH_TOKEN_DURATION_SECONDS`, `LOCKOUT_*`, `FRONTEND_URL`, `JWT_SECRET`) cargado desde `.env`.

## Endpoints del contexto

### `POST /api/v1/auth/sign-in`
- **Recurso**: `SigninResource { email, password }` (`validator::Validate` pide email y mínimo 6 caracteres).
- **Flujo**:
  1. Resuelve el `IdentityRepositoryImpl` dentro del esquema del tenant (utilizando `SET LOCAL search_path`).
  2. Usa `IdentityFacade` para verificar credenciales.
  3. Si hay fallos, incrementa contadores en `AccountLockoutService` (umbral y duración de `.env`).
  4. Si el usuario existe, genera JWT (`token`) + refresh token (`refresh_token`), extrae un `jti` nuevo, lo guarda en Redis y enlaza el refresh token con TTL (`REFRESH_TOKEN_DURATION_SECONDS`).
- **Respuesta**: `TokenResponse { token, refresh_token }`.
- **Errores**: 401 con mensaje genérico y trazas en servidor; cualquier bloqueo evita la generación de tokens.
-
### `POST /api/v1/auth/logout`
- **Recurso**: `LogoutResource { refresh_token }`.
- **Flujo**:
  1. Busca el user_id asociado al refresh token (hash en Redis).
  2. Borra el refresh token y elimina la sesión activa (clave `session:{user_id}`).
  3. No hace falta payload adicional, el token provisto se invalida automáticamente.
  4. Las sesiones de ese usuario se consideran cerradas tanto en el token actual como en futuros JWT vinculados.
- **Respuesta**: 200 OK si todo sale bien; 500 en fallos internos.

### `POST /api/v1/auth/refresh-token`
- **Recurso**: `RefreshTokenResource { refresh_token }`.
- **Flujo**:
  1. Valida el refresh token hash y obtiene `user_id`.
  2. Rota el token: elimina el antiguo, genera JWT nuevo y refresh token nuevo.
  3. Guarda el nuevo refresh token + JTI en Redis con el TTL compartido.
  4. Elimina cualquier cache que pudiera haber quedado (la sesión activa se actualiza con el nuevo JTI para evitar reuse).
- **Respuesta**: `TokenResponse` con el par renovado.

### `GET /api/v1/auth/verify`
- **Query**: `VerifyTokenResource { token }`.
- **Flujo**:
  1. Valida firma/exp (`JwtTokenService::validate_token`).
  2. Comprueba blacklist (`is_jti_blacklisted`).
  3. Verifica timestamp global de invalidación (del usuario) y que el token coincida con la sesión activa (`session:{user}`).
  4. Retorna `VerifyTokenResponse { is_valid, sub, error? }` con 200 y `is_valid=false` si el token no pasa las reglas de negocio.

### Google OAuth (Federation, usa servicios del contexto)
- **`GET /api/v1/auth/google`**: construye la URL de autorización de Google con el `client_id` del tenant; guarda `state` anti-CSRF en cookie HTTP-only y redirige.
- **`GET /api/v1/auth/google/callback`**: valida el `state`, consulta el tenant para `google_client_id/secret`, usa `GoogleOAuthClient` (con circuito breaker) para intercambiar código y obtener usuario; crea identidad si no existe y genera JWT+refresh exactamente como en sign-in; guarda los tokens en Redis y devuelve un código efímero.
- **`POST /api/v1/auth/google/claim`**: el frontend intercambia el código por el par `token/refresh_token` real (repo Redis `google_exchange:<code>` con TTL corto y borrado inmediato para evitar replays).

## Tokens y sesiones

- **JWT**: `JwtTokenService` usa `JWT_SECRET` para firmar tokens con `{ sub, exp, jti, iat }`. `session_duration_seconds` define `exp`.
- **Refresh tokens**: valores aleatorios hex (32 bytes) guardados en Redis hashed (`sha256`). Se asocian al usuario y se listan en `user_tokens:{user_id}` para poder borrarlos todos de una vez.
- **Sesiones en Redis**: `session:{user_id}` almacena el JTI activo. Durante refresh se actualiza (previniendo reuse de tokens viejos) y al reset de contraseña se invalidan (`revoke_all_user_sessions`):
  - Lista negra: `blacklist:{jti}` se crea cuando se revoca.
  - Timestamp: `invalidation_timestamp:{user}` impide que tokens antiguos pasen la verificación.

## Mecanismos de seguridad adicionales

- **Lockout de cuentas** (`LOCKOUT_THRESHOLD`, `LOCKOUT_DURATION_SECONDS`):
  - Cada fallo aumenta `login_failures:email[:ip]`. Al superar el umbral se crea `lockout:email[:ip]`.
  - Login satisfactoria resetea ambos contadores.
- **Rate limiting en middleware** (`shared/interfaces/rest/middleware.rs`):
  - IP global: 20 req/sec.
  - `/auth/sign-in`: 5 req/min.
  - `/identity/forgot-password`: 3 req/min.
  - Usa token bucket en Redis (`RedisRateLimiter`).
- **Circuit breaker en servicios externos**: `AppCircuitBreaker` protege el envío de correos y las llamadas a Google para evitar cascadas si el proveedor falla.

## Tablas de configuración clave

| Variable | Qué controla |
|----------|--------------|
| `JWT_SECRET` | Firma de JWT por tenant (cada tenant tiene su propio secret). |
| `SESSION_DURATION_SECONDS` | Duración estándar del JWT. |
| `REFRESH_TOKEN_DURATION_SECONDS` | TTL para refresh tokens almacenados en Redis. |
| `LOCKOUT_THRESHOLD` / `LOCKOUT_DURATION_SECONDS` | Regulan el mecanismo de bloqueo ante intentos fallidos. |
| `FRONTEND_URL` | Base usada en redirecciones de Google y confirmaciones para mostrar feedback. |
| `GOOGLE_REDIRECT_URI` | Debe coincidir con la ruta `/api/v1/auth/google/callback` del backend. |

## Guía para integradores/frontend

1. **Autenticación inicial**: llama a `/auth/sign-in` con `anon_key` del tenant en header (`apikey`/`Authorization`), guarda `token` (p.ej. en memoria o en cookie segura) y el `refresh_token`.
2. **Refrescar tokens automáticamente** antes de que expire el JWT (usa `refresh_token`, rota ambos).
3. **Logout**: envía el `refresh_token` para que el backend borre sesión y refresh.
4. **Verificar token**: útil para comprobar si el token sigue vigente antes de mostrar zonas protegidas (`/auth/verify`).
5. **Google OAuth**: sigue la secuencia `/auth/google` → usuario en Google → callback redirige con `code` → frontend llama `/auth/google/claim` y obtiene `token` + `refresh_token`.
6. **Errores y seguridad**: presenta mensajes genéricos al usuario, no expongas detalles del backend (ej. `forgot-password` siempre responde 200).

## Referencias de código

- `src/iam/authentication/interfaces/rest/controllers/authentication_controller.rs`
- `src/iam/authentication/application/command_services/authentication_command_service_impl.rs`
- `src/iam/authentication/application/query_services/authentication_query_service_impl.rs`
- `src/iam/authentication/infrastructure/services/jwt_token_service.rs`
- `src/iam/authentication/infrastructure/persistence/redis/redis_session_repository.rs`
- `src/shared/infrastructure/services/account_lockout.rs`
- `src/shared/infrastructure/services/rate_limiter.rs`
- `src/iam/federation/interfaces/rest/controllers/google_controller.rs`

## Ejemplos concretos

### Sign-in
**Request**
```json
POST /api/v1/auth/sign-in
{
  "email": "alice@example.com",
  "password": "secret123"
}
```
**Response** `200 OK`
```json
{
  "token": "eyJhbGciOiJIUzI1Ni...",
  "refresh_token": "b2c1d9..."
}
```
**Errores**
- `401`: credenciales inválidas o lockout activo.
- `400`: payload inválido.
- `429`: rate limit excedido.

### Refresh token
**Request**
```json
POST /api/v1/auth/refresh-token
{
  "refresh_token": "b2c1d9..."
}
```
**Response**
```json
{
  "token": "eyJhbGciOiJIUzI1Ni...",
  "refresh_token": "a5f3e2..."
}
```
**Errores**
- `401`: refresh token inválido/expirado.

### Logout
**Request**
```json
POST /api/v1/auth/logout
{
  "refresh_token": "a5f3e2..."
}
```
**Response** `200 OK` (sin body)
**Errores**
- `400`: payload inválido.
- `500`: error interno al borrar sesión.

### Verify token
**Request**
```
GET /api/v1/auth/verify?token=eyJhbGciOiJIUzI1Ni...
```
**Response** `200 OK`
```json
{
  "is_valid": true,
  "sub": "uuid-del-usuario"
}
```
Si el token no pasa validaciones de negocio, devuelve `is_valid=false` y un mensaje en `error`.
