#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendUrl(String);

impl FrontendUrl {
    pub fn new(value: String) -> Result<Self, String> {
        let normalized = value.trim().trim_end_matches('/').to_string();
        if normalized.is_empty() {
            return Err("frontend_url cannot be empty".to_string());
        }

        if normalized.starts_with("https://")
            || normalized.starts_with("http://localhost")
            || normalized.starts_with("http://127.0.0.1")
        {
            return Ok(Self(normalized));
        }

        Err(
            "frontend_url must use HTTPS, or HTTP only for localhost/127.0.0.1 in development"
                .to_string(),
        )
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
