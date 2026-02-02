# Messaging bounded context

## Visión general
`messaging` es el bounded context encargado de la salida de correos electrónicos (verificación, reset de contraseña) y actúa como capa de notificación para los otros bounded contexts (`identity`, principalmente). Se basa en principios de clean architecture: comandos de dominio, facades, servicios y adaptadores de infraestructura.

Componentes clave:
- `MessagingCommandService` + `MessagingCommandServiceImpl`: reciben comandos `SendEmailCommand` que contienen destinatario, asunto y cuerpo validados.
- `EmailSenderService` + `SmtpEmailSender`: adaptador SMTP que usa `lettre` y toma la configuración `SMTP_HOST/PORT/USERNAME/PASSWORD` desde `.env`.
- `MessagingFacade` + `MessagingFacadeImpl`: expone una interfaz simplificada para enviar correos desde otros contextos (p.ej. `IdentityCommandServiceImpl`).
- `AppCircuitBreaker`: protege el envío de correos para evitar saturación ante fallos del proveedor (abre un breaker después de 3 fallos en 60s).

## Flujo

1. `IdentityCommandServiceImpl` construye el `EmailService` que implementa `NotificationService`, inyectando un `MessagingFacadeImpl`.
2. El `EmailService` llama a `messaging_facade.send_email`, que crea el `SendEmailCommand` (validando email/subject/body) y lo pasa a `MessagingCommandService`.
3. `MessagingCommandServiceImpl` delega en el `EmailSenderService` (`SmtpEmailSender`), que construye el mensaje con `lettre`, respeta `circuit_breaker` y envía el correo.
4. El breaker llama `on_success`/`on_failure` según el resultado.
5. Si el servicio está abierto, se retorna un error (propagado hasta el controlador de identidad) y el endpoint responde 503 para que el frontend lo comunique oportunamente.

## Variables y configuración

| Variable | Propósito |
|----------|----------|
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD` | Configuración TLS para `lettre`. |
| `SMTP_PORT` por defecto 587 | Se filtra y valida como entero. |
| Circuit breaker (`AppCircuitBreaker`) | Se instancia en `main.rs` y se pasa a `SmtpEmailSender` al crear el servicio de mensajería. |

## Buenas prácticas

- Siempre maneja errores del servicio como “Service temporarily unavailable” (la capa de identidad ya devuelve 503).
- Reutiliza la misma instancia de `MessagingFacadeImpl` para mantener la configuración del breaker.
- Considera testear el `MessagingCommandService` con un fake `EmailSenderService` para evitar llamadas reales en CI.

## Referencias

- `src/messaging/domain/model/commands/send_email_command.rs`
- `src/messaging/application/command_services/messaging_command_service_impl.rs`
- `src/messaging/interfaces/acl/messaging_facade.rs`
- `src/messaging/application/acl/messaging_facade_impl.rs`
- `src/messaging/infrastructure/services/smtp_email_sender.rs`
- `src/shared/infrastructure/circuit_breaker.rs`
