# Shared bounded context

## Visión general
`shared` agrupa los elementos reutilizables que no pertenecen a un dominio específico sino que sirven a todos los bounded contexts: modelos auditables, infraestructura (Redis, rate limiting, circuit breaker) y middleware transversales (app state, errores, limitador global). Funciona como la columna vertebral del servicio.

## Componentes clave

- **Modelos**: existe un agregado abstracto auditable (`AuditableAbstractAggregateRoot`) y el `AuditableModel` (con `created_at`/`updated_at`) que otros agregados pueden incorporar para tener timestamps consistentes.
- **AppState** (`shared/interfaces/rest/app_state.rs`): centraliza `DatabaseConnection`, cliente Redis y parámetros como duraciones de sesión, TTLs, secretos (`jwt_secret`, `google_redirect_uri`), URIs de frontend y circuit breaker. Se clona antes de pasarlo a los routers/middleware de Axum.
- **Middleware**:
  - `rate_limit_middleware` (global, aplicado una sola vez en `main.rs`) usa `RedisRateLimiter` para aplicar límites por IP (global, sign-in, forgot password), tomando la IP real desde `ConnectInfo` y validando `X-Forwarded-For` solo si viene de proxies internos.
  - Todos los controladores dependen de `_middleware::tenant_resolver` (desde `tenancy`), pero `shared` provee errores estandarizados (`ErrorResponse`) para devolver JSON con mensaje/code.
- **Infraestructura**:
  - `shared/infrastructure/persistence/redis`: expone `connect()` que levanta `redis::Client` usando `REDIS_URL`.
  - `shared/infrastructure/services`:
    * `rate_limiter::RedisRateLimiter`: token bucket implementado en Lua con keys `...:tokens`/`:ts`; usado por el middleware para proteger rutas sensibles.
    * `account_lockout::AccountLockoutService`: gestiona fallos/locks en Redis usando claves `login_failures:<email[:ip]>` y `lockout:<email[:ip]>`; implementa la interfaz `AccountLockoutVerifier` que consume `AuthenticationCommandServiceImpl`.
  - `shared/infrastructure/circuit_breaker::AppCircuitBreaker`: protege integraciones externas (SMTP, Google OAuth) con estados `Closed/Open/HalfOpen`, historial de fallos y ventanas de tiempo.

## Buenas prácticas

1. Usa `AppState` clonando (`state.clone()`) antes de pasar al router para evitar referencias mutables compartidas; así se puede inyectar en los controladores con Axum.
2. Mantén métricas en Redis (rate limit, lockout, sesiones) limpias: las implementaciones ya manejan expiraciones y sets auxiliares.
3. Evita exponer errores internos directamente: `ErrorResponse` oculta detalles y usa códigos HTTP adecuados.

## Referencias

- `src/shared/domain/model/entities/auditable_model.rs`
- `src/shared/domain/model/aggregates/auditable_abstract_aggregate_root.rs`
- `src/shared/interfaces/rest/app_state.rs`
- `src/shared/interfaces/rest/error_response.rs`
- `src/shared/interfaces/rest/middleware.rs`
- `src/shared/infrastructure/persistence/redis/mod.rs`
- `src/shared/infrastructure/services/rate_limiter.rs`
- `src/shared/infrastructure/services/account_lockout.rs`
- `src/shared/infrastructure/circuit_breaker.rs`
