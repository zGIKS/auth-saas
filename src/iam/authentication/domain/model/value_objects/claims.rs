use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub tid: Uuid,
    pub role: String,
    pub exp: usize,
    pub jti: String,
    pub iat: usize,
}
