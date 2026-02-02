# Tenancy bounded context

## Visión general
`tenancy` gestiona el ciclo de vida de cada tenant y su metadata global. Controla la creación, consulta y configuración de secretos (JWT, OAuth) y estrategias de base de datos, y expone claves de acceso (`anon_key`) para que los servicios del tenant puedan resolver su identidad y contexto.

Depende de:

- `AppState` global (con `DatabaseConnection`, `jwt_secret` público, etc.).
- Repositorio PostgreSQL (`PostgresTenantRepository`) para persistir metadata en una tabla `tenants` dentro del esquema público.
- Inicializador de esquemas (`schema_initializer`) para crear el esquema y tablas de usuarios de cada tenant que usa la estrategia `shared`.
- Servicios de dominio que validan nombres y configuran `AuthConfig`.

## Endpoints del contexto

### `POST /api/v1/tenants`
- **Recurso**: `CreateTenantRequest { name, db_strategy_type, google_client_id?, google_client_secret? }`.
- **Validaciones**:
  - `name` alfanumérico/hyphen/underscore (3-30).
  - `db_strategy_type` es `shared` o `isolated`.
- **Flujo**:
  1. `CreateTenantCommand::new` valida el nombre, genera `TenantName`, selecciona estrategia (genera `tenant_<name>` para shared), genera JWT secret (128 hex chars) y crea `AuthConfig`.
  2. `TenantCommandServiceImpl` valida unicidad y guarda el tenant con `TenantRepository`.
  3. Si la estrategia es `Shared`, llama a `schema_initializer::initialize_tenant_schema` para crear el esquema y la tabla `users` del tenant.
  4. Devuelve `CreateTenantResponse { id, anon_key }`, donde `anon_key` es un JWT firmado con el secret global del backend (`state.jwt_secret`) con claims `{ iss: "saas-system", tenant_id, role: "anon" }`.
- **Errores**: 400 (validación), 409 (tenant ya existe), 500 (fallos de infraestructura o inicialización de esquema).

### `GET /api/v1/tenants/{id}`
- **Path**: `id` es UUID del tenant.
- **Flujo**:
  1. Consulta el repositorio y, si existe, regenera `anon_key` (sin almacenarlo) usando el JWT global del backend.
  2. Devuelve `TenantResource` con `db_strategy`, `auth_config`, `active` y `anon_key`.
  3. Si no se encuentra, responde 404.

## Estrategias de base de datos

- **Shared**: `tenant_<name>` dentro del PostgreSQL global. Se crea el esquema y la tabla `users` via `schema_initializer`. Las operaciones de identidad usan `SET LOCAL search_path` para aislar datos.
- **Isolated** (no implementada): placeholder con `connection_string` estática; actualmente cualquier ruta que selecciona `isolated` retorna error de configuración.

## Configuración y secretos

- Cada tenant tiene un `AuthConfig` con:
  - `jwt_secret` propio (mínimo 32 caracteres) usado por `tenant_resolver` y los endpoints de auth/identity para firmar/verificar tokens específicos del tenant.
  - Opcionales `google_client_id` y `google_client_secret` que habilitan Google OAuth.
- El backend guarda solo la metadata (JSON) de estrategia y auth config; el secret global (`state.jwt_secret`) firma las claves `anon_key`.

## Flujo común para nuevos tenants

1. **Crear** → `POST /api/v1/tenants` con nombre y estrategia.
2. **Inicializar** → `schema_initializer` crea esquema y tabla `users`.
3. **Consumir** → Frontend almacena `anon_key` del response y lo adjunta en headers (`apikey`/`Authorization`) para resolver el tenant. Luego puede usar `/auth` y `/identity` dentro del contexto resuelto.

## Referencias de código

- `src/tenancy/interfaces/rest/controllers/tenant_controller.rs`
- `src/tenancy/domain/model/commands/create_tenant_command.rs`
- `src/tenancy/application/command_services/tenant_command_service_impl.rs`
- `src/tenancy/infrastructure/persistence/postgres/postgres_tenant_repository.rs`
- `src/tenancy/infrastructure/schema_initializer.rs`
