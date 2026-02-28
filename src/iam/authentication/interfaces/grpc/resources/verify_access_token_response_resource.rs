use crate::{
    grpc::VerifyAccessTokenResponse,
    iam::authentication::domain::model::value_objects::claims::Claims,
};

#[derive(Debug, Clone)]
pub struct VerifyAccessTokenResponseResource {
    pub is_valid: bool,
    pub subject_id: String,
    pub jti: String,
    pub exp_epoch_seconds: u64,
    pub error_message: String,
}

impl VerifyAccessTokenResponseResource {
    pub fn valid(claims: Claims) -> Self {
        Self {
            is_valid: true,
            subject_id: claims.sub.to_string(),
            jti: claims.jti,
            exp_epoch_seconds: claims.exp as u64,
            error_message: String::new(),
        }
    }

    pub fn invalid(error_message: String) -> Self {
        Self {
            is_valid: false,
            subject_id: String::new(),
            jti: String::new(),
            exp_epoch_seconds: 0,
            error_message,
        }
    }
}

impl From<VerifyAccessTokenResponseResource> for VerifyAccessTokenResponse {
    fn from(value: VerifyAccessTokenResponseResource) -> Self {
        Self {
            is_valid: value.is_valid,
            subject_id: value.subject_id,
            jti: value.jti,
            exp_epoch_seconds: value.exp_epoch_seconds,
            error_message: value.error_message,
        }
    }
}
