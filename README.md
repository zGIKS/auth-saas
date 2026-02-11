# SaaS Auth Platform

Este repositorio contiene el backend del servicio de autenticación y autorización multi-tenant basado en Axum + SeaORM. Incluye contextos modulares para **tenancy**, **identity**, **authentication**, **federation**, **messaging** y utilidades compartidas. El servidor expone Swagger y endpoints REST para todo el ciclo de vida de usuarios, tokens, tenants y federación con Google.

## Características principales

- Configuración multi-tenant con estrategia `shared` (1 DB compartida, 1 schema por tenant) y claves `anon_key`.
- Registro, verificación de email, login, refresh/logout, recuperación de contraseña y Google OAuth.
- Módulo `AdminIdentity` para login admin, bootstrap inicial y recovery de credenciales.
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

## Deploy con Docker Compose (seguro)

Se agregó un despliegue con `docker-compose.yml` para correr solo la API. Postgres y Redis se toman desde variables de entorno (`DATABASE_URL` y `REDIS_URL`) apuntando a servicios externos.

1. Crea archivo de entorno para deploy:
   ```bash
   cp .env .env.backup
   ```
2. Edita `.env` con valores de deploy reales (`JWT_SECRET`, `DATABASE_URL`, `REDIS_URL`, `SMTP_PASSWORD`, `GOOGLE_REDIRECT_URI`).
3. Levanta los servicios:
   ```bash
   docker compose up -d --build
   ```
4. Verifica estado:
   ```bash
   docker compose ps
   docker compose logs -f app
   ```

Endurecimientos incluidos en Compose:

- API ejecuta como usuario no-root, `read_only`, `cap_drop: ALL` y `no-new-privileges`.
- La conexión a Postgres y Redis se resuelve vía env vars para usar servicios administrados fuera de Docker.
- Healthcheck en app para validar disponibilidad del proceso HTTP.

## Configuración clave

Variables imprescindibles (usa `.env`):

- `DATABASE_URL`, `REDIS_URL`: conexiones a Postgres y Redis.
- `JWT_SECRET`, `SESSION_DURATION_SECONDS`, `REFRESH_TOKEN_DURATION_SECONDS`: seguridad de tokens.
- `SWAGGER_ENABLED` (opcional): habilita o deshabilita Swagger en runtime.
- `FRONTEND_URL`, `GOOGLE_REDIRECT_URI`: rutas de callback y referencia para correos.
- `SMTP_*`: servidor SMTP para correos transaccionales.
- `LOCKOUT_THRESHOLD`, `LOCKOUT_DURATION_SECONDS`: control de bloqueo por intentos fallidos.

## Comandos operativos

- Ejecutar API:
  ```bash
  cargo run
  ```
- Crear admin inicial (solo primera vez):
  ```bash
  cargo run --bin admin_identity_bootstrap_cli
  ```
- Recuperar acceso admin (reemplaza username/password del admin unico):
  ```bash
  cargo run --bin admin_identity_recover_cli
  ```

Consulta `.env` para un set completo de ejemplo local.

## Endpoints principales

Además del Swagger (`/swagger-ui`), el backend expone:

- `/api/v1/tenants`: creación/consulta de tenants y sus claves anon.
- `/api/v1/admin/login`: login de administrador (JWT admin).
- `/api/v1/auth/*`: login, logout, refresh, verificación y federación con Google.
- `/api/v1/identity/*`: flujo completo de registro, confirmación y recuperación de contraseña.
- `/api/v1/auth/google/*`: inicio de OAuth y claim de tokens intercambiados en Redis.

`POST /api/v1/tenants` requiere JWT de admin en `Authorization: Bearer <token>`.

Para detalles completos de payloads, errores y flujos sugeridos revisa `docs/`.

## Documentación de bounded contexts

Los documentos en `docs/` explican cada módulo con visión general, endpoints, variables, errores, ejemplos y referencias de código:

- `identity-bc.md`: registro/confirmación, reset, envío de correos, interacción con Redis y lockout.
- `admin-identity-bc.md`: login admin, bootstrap/recovery por CLI y guard para creación de tenants.
- `commands.md`: comandos operativos de ejecución, bootstrap/recovery y calidad de código.
- `auth-bc.md`: login, refresh, logout, verificación de JWT y Google OAuth (incluye ejemplos de request/respuesta).
- `tenancy-bc.md`: creación de tenants, estrategia shared (schema por tenant), generación de `anon_key`.
- `federation-bc.md`: flujo completo de Google OAuth, CSRF y tokens temporales.
- `messaging-bc.md`: pipeline de mensajería, circuit breaker y configuración SMTP.
- `shared-bc.md`: AppState, middleware, rate limiter, circuit breaker y modelos auditables comunes.

## Recursos adicionales

- Swagger UI: `http://localhost:<PORT>/swagger-ui`
- OpenAPI JSON:  `http://localhost:<PORT>/api-docs/openapi.json`
- Carpetas clave: `src/iam`, `src/tenancy`, `src/messaging`, `src/shared`

Mantén la documentación en `docs/` sincronizada si agregas nuevos endpoints o variables.
