# Federation bounded context

## Visión general
`iam::federation` encapsula la autenticación federada con Google. Su objetivo es permitir que un usuario inicie sesión con Google y, si no existe, crear una identidad con proveedor `AuthProvider::Google`, sin manejar contraseñas en texto claro. Asegura la protección CSRF, la validación de tokens de Google y la transición segura de tokens (Google → backend → frontend).

Depende de:

- `GoogleOAuthClient` (servicio HTTP con `AppCircuitBreaker`) para intercambiar códigos por tokens y userinfo.
- `IdentityRepository` y `Identity` domain para verificar/crear usuarios Google.
- `JwtTokenService` y `RedisSessionRepository` (mismos que `authentication`) para emitir JWT/refreshtokens y llevar sesiones.
- `TokenExchangeRepository` (Redis) para entregar un código temporario seguido de `/auth/google/claim`.

## Endpoints

### `GET /api/v1/auth/google`
- Construye la URL de autorización con `client_id`, `redirect_uri`, `scope`, `access_type=offline` y `prompt=consent`.
- Genera `state` anti-CSRF, lo guarda en cookie segura (`oauth_state`), y redirige a Google.
- Consejo: el frontend muestra una pantalla de “Redirigiendo a Google” antes del redirect automático.

### `GET /api/v1/auth/google/callback`
- Valida el `state` comparando query y cookie. Si falla, redirige a `FRONTEND_URL/login?error=csrf_error`.
- Usa el `tenant_ctx` para obtener `google_client_id/secret`.
- Instancia `GoogleOAuthClient` y llama a `exchange_code` (circuit breaker). Si falla, escribe error y redirige con detalle.
- Si ya existe identidad con ese email pero distinto proveedor, falla con `ProviderMismatch`.
- Si no existe, crea la identidad con contraseña placeholder y provider Google.
- Genera token + refresh como en sign-in y guarda ambos en Redis.
- Llama a `TokenExchangeRepository::save` para emitir un `code` temporal (`google_exchange:<code>`). Redirige a `FRONTEND_URL/auth/google/callback?code=<code>` para que el frontend lo intercambie.

### `POST /api/v1/auth/google/claim`
- Recibe `ClaimTokenRequest { code }` validado.
- Busca y borra el `google_exchange:<code>` en Redis; si existe, retorna `ClaimTokenResponse { token, refresh_token }`.
- Si no existe o expiró (TTL 60s), devuelve 400.
- La respuesta contiene el par JWT + refresh del backend listo para usar.

## Seguridad y tokens

- Todos los tokens emitidos están ligados al tenant del `TenantContext` y usan su `jwt_secret`.
- Los tokens de Google nunca se almacenan directamente; solo se guardan los tokens emitidos por backend en Redis por unos segundos mediante `TokenExchangeRepository`.
- El intercambio invalida el código en cuanto se llama a `/claim` para evitar replays.
- El circuito (`AppCircuitBreaker`) evita que fallos prolongados de Google rompan el backend o el envío de correos.

## Flujo en el frontend

1. El usuario clickea “Continuar con Google” → Frontend hace GET `/api/v1/auth/google`.
2. Google redirige a `/api/v1/auth/google/callback` en el backend (ya con `state` cookie).
3. Backend, tras obtener tokens Google, redirige a `FRONTEND_URL/auth/google/callback?code=<código efímero>`.
4. Frontend POST `/api/v1/auth/google/claim` con ese `code`.
5. Backend responde con `{ token, refresh_token }` que el frontend usa igual que el login estándar.

## Referencias

- `src/iam/federation/interfaces/rest/controllers/google_controller.rs`
- `src/iam/federation/application/services/google_federation_service.rs`
- `src/iam/federation/infrastructure/services/google_oauth_client.rs`
- `src/iam/federation/infrastructure/persistence/redis/token_exchange_repository_impl.rs`
