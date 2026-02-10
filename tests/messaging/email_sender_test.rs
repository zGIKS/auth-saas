use auth_service::messaging::domain::model::value_objects::{
    body::Body, email_address::EmailAddress, subject::Subject,
};
use auth_service::messaging::domain::services::email_sender_service::EmailSenderService;
use auth_service::messaging::infrastructure::services::smtp_email_sender::SmtpEmailSender;
use auth_service::messaging::domain::error::MessagingError;
use auth_service::shared::infrastructure::circuit_breaker::create_circuit_breaker;
use dotenvy::dotenv;

#[tokio::test]
async fn test_send_email_integration() {
    dotenv().ok();

    // Skip test if SMTP config is missing (so CI doesn't fail without creds)
    if std::env::var("SMTP_PASSWORD").is_err() {
        println!("Skipping email test: SMTP_PASSWORD not set");
        return;
    }

    let sender =
        SmtpEmailSender::new(create_circuit_breaker()).expect("Failed to create SMTP sender");

    // Primary destination for integration tests
    let primary_to_addr = std::env::var("SMTP_TO").unwrap_or_else(|_| "test@example.com".to_string());
    let fallback_to_addr = std::env::var("SMTP_FALLBACK_TO")
        .or_else(|_| std::env::var("SMTP_FROM"))
        .unwrap_or_else(|_| primary_to_addr.clone());

    let to = EmailAddress::new(primary_to_addr.clone()).unwrap();
    let subject = Subject::new("Integration Test Email".to_string()).unwrap();
    let body =
        Body::new("This is a test email from the auth-service integration test.".to_string())
            .unwrap();

    let result = sender.send(&to, &subject, &body).await;

    match result {
        Ok(_) => println!("Email sent successfully to {}", primary_to_addr),
        Err(MessagingError::SendError(message)) if message.contains("You can only send testing emails to your own email address") => {
            if fallback_to_addr == primary_to_addr {
                println!("Skipping email test due to Resend sandbox restriction: {}", message);
                return;
            }

            let fallback_to = match EmailAddress::new(fallback_to_addr.clone()) {
                Ok(value) => value,
                Err(_) => {
                    println!(
                        "Skipping email test: fallback recipient is invalid and sandbox blocked primary"
                    );
                    return;
                }
            };
            let fallback_result = sender.send(&fallback_to, &subject, &body).await;

            match fallback_result {
                Ok(_) => println!(
                    "Primary recipient blocked by sandbox; email sent to fallback {}",
                    fallback_to_addr
                ),
                Err(e) => panic!("Failed to send email (including fallback): {:?}", e),
            }
        }
        Err(e) => panic!("Failed to send email: {:?}", e),
    }
}
