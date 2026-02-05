# SaaS Auth Platform

Este repositorio contiene el backend del servicio de autenticación y autorización multi-tenant basado en Axum + SeaORM. Incluye contextos modulares para **tenancy**, **identity**, **authentication**, **federation**, **messaging** y utilidades compartidas. El servidor expone Swagger y endpoints REST para todo el ciclo de vida de usuarios, tokens, tenants y federación con Google.

## Características principales

- Configuración multi-tenant con estrategia `isolated` y claves `anon_key`.
- Registro, verificación de email, login, refresh/logout, recuperación de contraseña y Google OAuth.
- Redis para sesiones, refresh tokens, rate limiting y lockout.
- Circuit breaker y mensajería SMTP para notificaciones seguras.
- Documentación OpenAPI pública (`/swagger-ui` y `/api-docs/openapi.json`).

## Requisitos

- Rust 1.70+
- Cargo
- PostgreSQL y Redis accesibles con las URLs definidas en `.env`

## Instalación y ejecución

1. Clona el repositorio:
   ```bash
   git clone <repository-url> auth-service
   cd auth-service
   ```
2. Ajusta `.env` con las variables obligatorias (ejemplo mínimo en `.env`).
3. Compila y ejecuta:
   ```bash
   cargo run
   ```
4. Accede al servidor en `http://localhost:<PORT>` (por defecto 8081 según `.env`).

## Configuración clave

Variables imprescindibles (usa `.env`):

- `DATABASE_URL`, `REDIS_URL`: conexiones a Postgres y Redis.
- `JWT_SECRET`, `SESSION_DURATION_SECONDS`, `REFRESH_TOKEN_DURATION_SECONDS`: seguridad de tokens.
- `FRONTEND_URL`, `GOOGLE_REDIRECT_URI`: rutas de callback y referencia para correos.
- `SMTP_*`: servidor SMTP para correos transaccionales.
- `LOCKOUT_THRESHOLD`, `LOCKOUT_DURATION_SECONDS`: control de bloqueo por intentos fallidos.
- `RATE_LIMIT_DISABLED` (opcional): desactiva el rate limiter en desarrollo/local.
- `VAULT_ADDR`, `VAULT_ROLE_ID`, `VAULT_SECRET_ID`: acceso a HashiCorp Vault (AppRole).
- `VAULT_NAMESPACE` (opcional), `VAULT_KV_MOUNT` (opcional, default `secret`): configuración avanzada de Vault.
- `TENANT_DB_IMAGE`, `TENANT_DB_NETWORK`, `TENANT_DB_HOST`: configuración de contenedores DB por tenant.
- `TENANT_DB_USER`, `TENANT_DB_NAME_PREFIX`: credenciales base para DB aisladas.
- `TENANT_DB_MEMORY_MB`, `TENANT_DB_CPU_CORES`: límites de recursos para mini‑dockers (opcional).

Consulta `.env` para un set completo de ejemplo local.

## Endpoints principales

Además del Swagger (`/swagger-ui`), el backend expone:

- `/api/v1/tenants`: creación/consulta de tenants y sus claves anon.
- `/api/v1/auth/*`: login, logout, refresh, verificación y federación con Google.
- `/api/v1/identity/*`: flujo completo de registro, confirmación y recuperación de contraseña.
- `/api/v1/auth/google/*`: inicio de OAuth y claim de tokens intercambiados en Redis.

Para detalles completos de payloads, errores y flujos sugeridos revisa `docs/`.

## Documentación de bounded contexts

Los documentos en `docs/` explican cada módulo con visión general, endpoints, variables, errores, ejemplos y referencias de código:

- `identity-bc.md`: registro/confirmación, reset, envío de correos, interacción con Redis y lockout.
- `auth-bc.md`: login, refresh, logout, verificación de JWT y Google OAuth (incluye ejemplos de request/respuesta).
- `tenancy-bc.md`: creación de tenants, estrategia de DB aislada, generación de `anon_key`.
- `federation-bc.md`: flujo completo de Google OAuth, CSRF y tokens temporales.
- `messaging-bc.md`: pipeline de mensajería, circuit breaker y configuración SMTP.
- `shared-bc.md`: AppState, middleware, rate limiter, circuit breaker y modelos auditables comunes.

## Recursos adicionales

- Swagger UI: `http://localhost:<PORT>/swagger-ui`
- OpenAPI JSON:  `http://localhost:<PORT>/api-docs/openapi.json`
- Carpetas clave: `src/iam`, `src/tenancy`, `src/messaging`, `src/shared`

Mantén la documentación en `docs/` sincronizada si agregas nuevos endpoints o variables.
