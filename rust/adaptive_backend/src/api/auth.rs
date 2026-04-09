//! Auth Middleware - JWT authentication from Master Tunnel

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub module_id: String,
    pub exp: usize,
    pub role: String,
}

pub struct AuthMiddleware {
    secret_key: Arc<String>,
}

impl AuthMiddleware {
    pub fn new(secret_key: String) -> Self {
        Self {
            secret_key: Arc::new(secret_key),
        }
    }

    pub async fn authenticate(
        &self,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let auth_header = request
            .headers()
            .get("Authorization")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        if !auth_str.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let token = auth_str.trim_start_matches("Bearer ");

        let key = DecodingKey::from_secret(self.secret_key.as_bytes());
        let validation = Validation::default();

        let _claims = decode::<Claims>(token, &key, &validation)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(next.run(request).await)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, StatusCode> {
        let key = DecodingKey::from_secret(self.secret_key.as_bytes());
        let validation = Validation::default();

        let token_data = decode::<Claims>(token, &key, &validation)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[test]
    fn test_auth_creation() -> anyhow::Result<()> {
        let auth = AuthMiddleware::new("test_secret".to_string());
        assert_eq!(*auth.secret_key, "test_secret");
        Ok(())
    }

    #[test]
    fn test_verify_valid_token() -> anyhow::Result<()> {
        let auth = AuthMiddleware::new("test_secret".to_string());

        let claims = Claims {
            sub: "user1".to_string(),
            module_id: "adaptive".to_string(),
            exp: 9999999999,
            role: "admin".to_string(),
        };

        let key = EncodingKey::from_secret("test_secret".as_bytes());
        let token = encode(&Header::default(), &claims, &key)?;

        let verified = auth.verify_token(&token).map_err(|e| anyhow::anyhow!("verify failed: {:?}", e))?;
        assert_eq!(verified.sub, "user1");
        assert_eq!(verified.role, "admin");

        Ok(())
    }

    #[test]
    fn test_verify_invalid_token() -> anyhow::Result<()> {
        let auth = AuthMiddleware::new("test_secret".to_string());

        let result = auth.verify_token("invalid_token");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_verify_wrong_secret() -> anyhow::Result<()> {
        let auth = AuthMiddleware::new("wrong_secret".to_string());

        let claims = Claims {
            sub: "user1".to_string(),
            module_id: "adaptive".to_string(),
            exp: 9999999999,
            role: "admin".to_string(),
        };

        let key = EncodingKey::from_secret("test_secret".as_bytes());
        let token = encode(&Header::default(), &claims, &key)?;

        let result = auth.verify_token(&token);
        assert!(result.is_err());

        Ok(())
    }
}
