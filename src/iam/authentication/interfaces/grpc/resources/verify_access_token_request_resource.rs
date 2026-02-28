use tonic::Status;

use crate::grpc::VerifyAccessTokenRequest;

#[derive(Debug, Clone)]
pub struct VerifyAccessTokenRequestResource {
    pub access_token: String,
}

impl TryFrom<VerifyAccessTokenRequest> for VerifyAccessTokenRequestResource {
    type Error = Status;

    fn try_from(value: VerifyAccessTokenRequest) -> Result<Self, Self::Error> {
        let access_token = value.access_token.trim().to_string();

        if access_token.is_empty() {
            return Err(Status::invalid_argument("access_token must not be empty"));
        }

        Ok(Self { access_token })
    }
}
