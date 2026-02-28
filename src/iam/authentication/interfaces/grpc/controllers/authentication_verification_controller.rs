use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::{
    grpc::{
        VerifyAccessTokenRequest, VerifyAccessTokenResponse,
        authentication_verification_service_server::AuthenticationVerificationService,
    },
    iam::authentication::domain::services::authentication_command_service::AuthenticationQueryService,
};

use crate::iam::authentication::interfaces::grpc::resources::{
    verify_access_token_request_resource::VerifyAccessTokenRequestResource,
    verify_access_token_response_resource::VerifyAccessTokenResponseResource,
};

pub struct AuthenticationVerificationGrpcController<Q>
where
    Q: AuthenticationQueryService,
{
    query_service: Arc<Q>,
}

impl<Q> AuthenticationVerificationGrpcController<Q>
where
    Q: AuthenticationQueryService,
{
    pub fn new(query_service: Arc<Q>) -> Self {
        Self { query_service }
    }

    fn is_infrastructure_error(error_message: &str) -> bool {
        let normalized = error_message.to_lowercase();

        [
            "redis",
            "connection",
            "timeout",
            "timed out",
            "broken pipe",
            "dns",
            "transport",
            "unavailable",
            "io error",
            "network",
        ]
        .iter()
        .any(|pattern| normalized.contains(pattern))
    }
}

#[tonic::async_trait]
impl<Q> AuthenticationVerificationService for AuthenticationVerificationGrpcController<Q>
where
    Q: AuthenticationQueryService + 'static,
{
    async fn verify_access_token(
        &self,
        request: Request<VerifyAccessTokenRequest>,
    ) -> Result<Response<VerifyAccessTokenResponse>, Status> {
        let request_resource = VerifyAccessTokenRequestResource::try_from(request.into_inner())?;

        match self
            .query_service
            .verify_token(&request_resource.access_token)
            .await
        {
            Ok(claims) => Ok(Response::new(
                VerifyAccessTokenResponseResource::valid(claims).into(),
            )),
            Err(error) => {
                let error_message = error.to_string();

                if Self::is_infrastructure_error(&error_message) {
                    return Err(Status::internal(error_message));
                }

                Ok(Response::new(
                    VerifyAccessTokenResponseResource::invalid(error_message).into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use uuid::Uuid;

    use crate::iam::authentication::domain::{
        model::value_objects::claims::Claims,
        services::authentication_command_service::AuthenticationQueryService,
    };

    use super::*;

    #[derive(Clone)]
    struct FakeAuthenticationQueryService {
        mode: FakeMode,
    }

    #[derive(Clone)]
    enum FakeMode {
        Valid,
        Invalid,
        InfraError,
    }

    #[async_trait::async_trait]
    impl AuthenticationQueryService for FakeAuthenticationQueryService {
        async fn verify_token(&self, _token: &str) -> Result<Claims, Box<dyn Error + Send + Sync>> {
            match self.mode {
                FakeMode::Valid => Ok(Claims {
                    sub: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
                    role: "user".to_string(),
                    exp: 1_900_000_000,
                    jti: "jti-123".to_string(),
                    iat: 1_800_000_000,
                }),
                FakeMode::Invalid => Err("Token has been revoked".into()),
                FakeMode::InfraError => Err("redis connection refused".into()),
            }
        }
    }

    #[tokio::test]
    async fn verify_access_token_returns_valid_payload() {
        let controller = AuthenticationVerificationGrpcController::new(Arc::new(
            FakeAuthenticationQueryService {
                mode: FakeMode::Valid,
            },
        ));

        let response = controller
            .verify_access_token(Request::new(VerifyAccessTokenRequest {
                access_token: "token".to_string(),
            }))
            .await
            .expect("grpc response should succeed")
            .into_inner();

        assert!(response.is_valid);
        assert_eq!(response.subject_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(response.jti, "jti-123");
        assert_eq!(response.exp_epoch_seconds, 1_900_000_000);
        assert!(response.error_message.is_empty());
    }

    #[tokio::test]
    async fn verify_access_token_returns_invalid_payload_for_business_error() {
        let controller = AuthenticationVerificationGrpcController::new(Arc::new(
            FakeAuthenticationQueryService {
                mode: FakeMode::Invalid,
            },
        ));

        let response = controller
            .verify_access_token(Request::new(VerifyAccessTokenRequest {
                access_token: "token".to_string(),
            }))
            .await
            .expect("grpc response should succeed")
            .into_inner();

        assert!(!response.is_valid);
        assert!(response.subject_id.is_empty());
        assert!(response.jti.is_empty());
        assert_eq!(response.exp_epoch_seconds, 0);
        assert_eq!(response.error_message, "Token has been revoked");
    }

    #[tokio::test]
    async fn verify_access_token_returns_internal_status_for_infrastructure_error() {
        let controller = AuthenticationVerificationGrpcController::new(Arc::new(
            FakeAuthenticationQueryService {
                mode: FakeMode::InfraError,
            },
        ));

        let error = controller
            .verify_access_token(Request::new(VerifyAccessTokenRequest {
                access_token: "token".to_string(),
            }))
            .await
            .expect_err("grpc response should fail");

        assert_eq!(error.code(), tonic::Code::Internal);
    }
}
