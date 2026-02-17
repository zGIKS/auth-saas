use asphanyx::messaging::domain::error::MessagingError;
use asphanyx::messaging::domain::model::value_objects::{
    body::Body, email_address::EmailAddress, subject::Subject,
};
use asphanyx::messaging::domain::services::email_sender_service::EmailSenderService;
use asphanyx::messaging::infrastructure::services::smtp_email_sender::SmtpEmailSender;
use asphanyx::shared::infrastructure::circuit_breaker::create_circuit_breaker;
use dotenvy::dotenv;

#[tokio::test]
async fn test_send_email_integration() {
    dotenv().ok();

    // Map TEST_SMTP_* variables to standard SMTP_* variables
    let test_vars = ["HOST", "PORT", "USERNAME", "PASSWORD", "FROM", "TO"];
    for var in test_vars {
        if let Ok(val) = std::env::var(format!("TEST_SMTP_{}", var)) {
            unsafe {
                std::env::set_var(format!("SMTP_{}", var), val);
            }
        }
    }

    // Skip test if SMTP password is missing (checks after mapping)
    if std::env::var("SMTP_PASSWORD").is_err() {
        println!("Skipping email test: SMTP_PASSWORD (or TEST_SMTP_PASSWORD) not set");
        return;
    }

    let sender =
        SmtpEmailSender::new(create_circuit_breaker()).expect("Failed to create SMTP sender");

    // Primary destination for integration tests - Use Resend's safe test address by default
    let primary_to_addr =
        std::env::var("SMTP_TO").unwrap_or_else(|_| "delivered@resend.dev".to_string());
    let fallback_to_addr = std::env::var("SMTP_FALLBACK_TO")
        .or_else(|_| std::env::var("SMTP_FROM"))
        .unwrap_or_else(|_| "delivered@resend.dev".to_string());

    // Clean address to avoid validation errors like "user@gmail..com" if they exist in env
    let primary_to_addr = primary_to_addr.replace("..", ".");

    let to = EmailAddress::new(primary_to_addr.clone()).expect("Invalid SMTP_TO address");
    let subject = Subject::new("Integration Test Email".to_string()).unwrap();
    let body =
        Body::new("This is a test email from the asphanyx integration test.".to_string())
            .unwrap();

    let result = sender.send(&to, &subject, &body).await;

    match result {
        Ok(_) => println!("Email sent successfully to {}", primary_to_addr),
        Err(MessagingError::SendError(message))
            if message.contains("You can only send testing emails to your own email address") =>
        {
            if fallback_to_addr == primary_to_addr {
                println!(
                    "Skipping email test due to Resend sandbox restriction: {}",
                    message
                );
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
