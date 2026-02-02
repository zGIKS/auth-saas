# Identity bounded context

## Visión general
El bounded context `iam::identity` se encarga de todo el ciclo de vida de una identidad (usuario/email) dentro de un tenant:

- **Registro**: valida email y contraseña, guarda un `PendingIdentity` en Redis y envía un correo de confirmación.
- **Confirmación**: consume el token de verificación (también guardado en Redis) para persistir la identidad definitiva.
- **Recuperación de contraseña**: envía un link temporal que permite redefinir la contraseña y, al hacerlo, invalida las sesiones existentes.
- **Integración con otros servicios**: utiliza el servicio de mensajería (SMTP + circuito) y el repositorio de Redis/Postgres del tenant actual, resolviendo el esquema con `SET LOCAL search_path`.

El contexto depende de:

- `State` compartido (`AppState`) con conexión a Postgres, Redis, TTLs y secretos (`SESSION_DURATION_SECONDS`, `FRONTEND_URL`, etc.).
- Repositorios específicos para Redis/Postgres que garantizan aislamiento por tenant (`IdentityRepositoryImpl` + `PendingIdentityRepositoryImpl` + `PasswordResetTokenRepositoryImpl`).
- Servicios auxiliares como `SmtpEmailSender`, `MessagingFacadeImpl` y `SessionInvalidationServiceImpl`.

## Endpoints de este contexto

### `POST /api/v1/auth/sign-up`
- **Recurso**: `RegisterIdentityRequest { email, password }` (`validation email + longitud 6-72`).
- **Flujo**:
  1. Valida payload.
  2. Usa `IdentityRepositoryImpl` del esquema del tenant.
  3. Guarda `PendingIdentity` (email, hash, provider) en Redis con TTL (`PENDING_REGISTRATION_TTL_SECONDS`).
  4. Genera token de verificación y construye el link `FRONTEND_URL/verify?token=...`.
  5. Envía correo con el link usando SMTP (con circuito breaker).
  6. Devuelve `201` con mensaje genérico (`RegisterIdentityResponse`).
- **Consideraciones**: si ya existe un pending token, se elimina el anterior para evitar race conditions o múltiples tokens válidos.

### `GET /api/v1/identity/confirm-registration`
- **Query**: `token` (validado via `ConfirmEmailQuery`).
- **Flujo**:
  1. Crea `ConfirmRegistrationCommand` y resuelve el `PendingIdentity`.
  2. Reconstruye la identidad (hash de contraseña, provider).
  3. Persiste en Postgres dentro del esquema del tenant (con `SET LOCAL search_path`).
  4. Borra el token en Redis.
  5. Redirige a `FRONTEND_URL/email-verified?success=true` o a la ruta de error con mensaje.
- **Seguridad**: usa el valor `FRONTEND_URL` configurado para construir las redirecciones; falla con 302 y mensaje codificado en caso de token inválido o servicio no disponible.

### `POST /api/v1/identity/forgot-password`
- **Recurso**: `RequestPasswordResetRequest { email }`.
- **Flujo**:
  1. Verifica existencia de identidad (pero responde `200` igual si no existe para no revelar nada).
  2. Genera un `VerificationToken` con TTL (`PASSWORD_RESET_TTL_SECONDS`).
  3. Usa un bloqueo distribuido (clave `password_reset_lock:<email>`) y borra tokens previos.
  4. Guarda en Redis la asociación `token_hash` ⇄ `email`.
  5. Envía correo con `FRONTEND_URL/reset-password?token=...`.
  6. Devuelve mensaje genérico.

### `POST /api/v1/identity/reset-password`
- **Recurso**: `ResetPasswordRequest { token, new_password }`.
- **Flujo**:
  1. Busca el `email` asociado al token hash y valida que sea el más reciente.
  2. Recupera la identidad del tenant.
  3. Hashea la nueva contraseña y la guarda en Postgres.
  4. Elimina el token y, como medida de seguridad, invalida todas las sesiones del usuario mediante `SessionInvalidationService`, que revoca JTIs y refresh tokens en Redis.
  5. Devuelve `ResetPasswordResponse` con mensaje de éxito o error específico si el token es inválido.

## Infraestructura y mecanismos reutilizados

- **Repositorios Redis**:
  - `PendingIdentityRepositoryImpl`: guarda `pending_identity:<token_hash>` y `pending_email:<email>` (TTL = `PENDING_REGISTRATION_TTL_SECONDS`).
  - `PasswordResetTokenRepositoryImpl`: usa bloqueo `password_reset_lock:<email>` y mantiene token activo + referencia por email para evitar tokens obsoletos.
  - `RedisSessionRepository`: reutilizado para invalidar sesiones cuando cambia la contraseña y para la gestión de JTIs/refresh tokens (usado también por el contexto de Auth).

- **SMTP + mensajería**:
  - `SmtpEmailSender` usa `SMTP_{HOST,PORT,USERNAME,PASSWORD}` y un `AppCircuitBreaker` para evitar saturar el proveedor.
  - Se construye un `EmailService` que implementa `NotificationService`, y se inyecta en `IdentityCommandServiceImpl`.

- **Validaciones y reglas de negocio**:
  - `Password::new` + `Email::new` aplican criterios de validación de dominio.
  - El helper `validate_frontend_url` exige HTTPS en producción (HTTP solo en localhost/127.0.0.1).
  - `IdentityCommandServiceImpl` asegura que solo haya un token de verificación activo por email, maneja errores y encapsula la lógica de sesión (revocación tras reset).

## Variables de entorno relevantes

| Variable | Propósito |
|----------|-----------|
| `FRONTEND_URL` | Base usada para los links de verificación y reset (ej. `http://localhost:3000`). |
| `PENDING_REGISTRATION_TTL_SECONDS` | TTL de tokens de confirmación. |
| `PASSWORD_RESET_TTL_SECONDS` | TTL de tokens de reset. |
| `SESSION_DURATION_SECONDS`, `REFRESH_TOKEN_DURATION_SECONDS` | Duraciones que se pasan a los repositorios/session invalidation. |
| `SMTP_*` | Configurar envío de correos. |
| `GOOGLE_REDIRECT_URI` | Solo se usa en federation (OAuth), pero el frontend que integra identity debe respetar el mismo dominio. |

## Buenas prácticas para el frontend / SDK

1. **Enviar siempre el `anon_key`/tenant_id**: antes de consumir cualquier endpoint de `iam::identity`, asegúrate de incluir en los headers `apikey` o `Authorization: Bearer <anon_key>`. Si no tienes `anon_key`, puedes obtenerlo con `GET /api/v1/tenants/{id}`.
2. **Mantener estado de confirmación**: redirige al usuario a una vista “Verificando correo” tras hacer `sign-up`, y usa las rutas `email-verified|email-verification-failed` para mostrar feedback (los redirects incluyen mensajes codificados).
3. **Manejo seguro de tokens**: en procesos de reset, el token proviene de la URL (`/reset-password?token=...`). El SDK debe enviarlo inmediatamente, preferiblemente sobre POST y nunca almacenarlo en localStorage sin protección.
4. **Errores genéricos**: nunca reveles detalles (p.ej., en `forgot-password` siempre se responde exitoso). El SDK solo muestra mensajes genéricos y deja que el backend decida cuándo informar errores internos.

## Flujo recomendado (mapa rápido)

1. **Registrar** → `POST /api/v1/auth/sign-up` → `RegisterIdentityResponse`.
2. **Confirmar** → abrir link de correo → `GET /api/v1/identity/confirm-registration?token=...` → redirect a frontend.
3. **Login** → `POST /api/v1/auth/sign-in` → guardar `token` + `refresh_token`.
4. **Reset**: `POST /api/v1/identity/forgot-password` → correo → `POST /api/v1/identity/reset-password`.

## Referencias de código

- `src/iam/identity/interfaces/rest/controllers/identity_controller.rs`
- `src/iam/identity/application/command_services/identity_command_service_impl.rs`
- `src/iam/identity/infrastructure/persistence/redis`
- `src/messaging/infrastructure/services/smtp_email_sender.rs`
- `src/shared/infrastructure/circuit_breaker.rs`

## Ejemplos concretos

### Registro
**Request**
```json
POST /api/v1/auth/sign-up
{
  "email": "user@example.com",
  "password": "muySegura123"
}
```
**Response** `201 Created`
```json
{
  "message": "Identity registered successfully. Please check your email to verify your account."
}
```
**Errores**
- `400`: invalid email/password o token faltante.
- `409`: email repetido (no expuesto en endpoint, se responde 400 genérico).

### Confirmación
**Request**
```
GET /api/v1/identity/confirm-registration?token=abc123
```
**Comportamiento**
- `302` a `FRONTEND_URL/email-verified?success=true` al confirmar.
- `302` a `/email-verification-failed` con `error=invalid_token` o `error=service_unavailable`.

### Reset de contraseña
**Request**
```json
POST /api/v1/identity/reset-password
{
  "token": "abc123",
  "new_password": "NuevaPass456!"
}
```
**Response** `200 OK`
```json
{
  "message": "Password has been reset successfully."
}
```
**Errores**
- `400`: token inválido o expirado.
- `500`: error interno (p. ej., Redis inaccesible).
