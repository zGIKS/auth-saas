# Tenancy bounded context

## Visión general
`tenancy` gestiona el ciclo de vida de cada tenant y su metadata global. Controla la creación, consulta y configuración de secretos (JWT, OAuth) y la estrategia de base de datos compartida por schemas, y expone claves de acceso (`anon_key`) para que los servicios del tenant puedan resolver su identidad y contexto.

Depende de:

- `AppState` global (con `DatabaseConnection`, `jwt_secret` público, etc.).
- Repositorio PostgreSQL (`PostgresTenantRepository`) para persistir metadata en una tabla `tenants` dentro del esquema público.
- Servicios de dominio que validan nombres y configuran `AuthConfig`.

## Endpoints del contexto

### `POST /api/v1/tenants`
- **Recurso**: `CreateTenantRequest { name, google_client_id?, google_client_secret? }`.
- **Auth requerida**: JWT de admin en `Authorization: Bearer <token>`.
- **Validaciones**:
  - `name` alfanumérico/hyphen/underscore (3-30).
- **Flujo**:
  1. Middleware `require_admin_jwt` valida firma y que `sub` exista en `admin_accounts`.
  2. El controlador normaliza el nombre y deriva el `schema` del tenant (`tenant_<name_normalizado>`).
  3. Inicializa el schema en la base de datos compartida y crea tablas del tenant (tabla `users`).
  4. `CreateTenantCommand::new` valida nombre y schema, genera `TenantName`, construye estrategia shared, genera JWT secret (128 hex chars) y crea `AuthConfig`.
  5. `TenantCommandServiceImpl` valida unicidad y guarda el tenant con `TenantRepository`.
  6. Devuelve `CreateTenantResponse { id, anon_key }`, donde `anon_key` es un JWT firmado con el secret global del backend (`state.jwt_secret`) con claims `{ iss: "saas-system", tenant_id, role: "anon" }`.
- **Errores**: 400 (validación), 401 (JWT admin faltante/inválido), 409 (tenant ya existe), 500 (fallos de infraestructura).

### `GET /api/v1/tenants/{id}`
- **Path**: `id` es UUID del tenant.
- **Flujo**:
  1. Consulta el repositorio y, si existe, regenera `anon_key` (sin almacenarlo) usando el JWT global del backend.
  2. Devuelve `TenantResource` con `db_strategy_type`, `auth_config`, `active` y `anon_key`.
  3. Si no se encuentra, responde 404.

## Estrategia de base de datos

- **Shared (schema-per-tenant)**: todos los tenants usan la misma DB de Postgres y cada tenant tiene su propio schema. La capa de identidad/auth/federation abre conexión con `search_path=<schema>,public`.

## Configuración y secretos

- Cada tenant tiene un `AuthConfig` con:
  - `jwt_secret` propio (mínimo 32 caracteres) usado por `tenant_resolver` y los endpoints de auth/identity para firmar/verificar tokens específicos del tenant.
  - Opcionales `google_client_id` y `google_client_secret` que habilitan Google OAuth.
- El backend guarda solo la metadata (JSON) de estrategia y auth config; el secret global (`state.jwt_secret`) firma las claves `anon_key`.

## Flujo común para nuevos tenants

1. **Crear** → autentica admin en `POST /api/v1/admin/login` y usa ese JWT para llamar `POST /api/v1/tenants`; el backend crea schema + tablas del tenant en la DB compartida.
2. **Consumir** → Frontend almacena `anon_key` del response y lo adjunta en headers (`apikey`/`Authorization`) para resolver el tenant. Luego puede usar `/auth` y `/identity` dentro del contexto resuelto.

## Referencias de código

- `src/tenancy/interfaces/rest/controllers/tenant_controller.rs`
- `src/tenancy/domain/model/commands/create_tenant_command.rs`
- `src/tenancy/application/command_services/tenant_command_service_impl.rs`
- `src/tenancy/infrastructure/persistence/postgres/postgres_tenant_repository.rs`
