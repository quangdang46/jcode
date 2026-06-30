pub mod auth;
pub mod endpoint;
pub mod framing;
pub mod model_ref;
pub mod protocol;
pub mod route;
pub mod schema;
pub mod transport;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
    #[tokio::test]
    async fn test_auth_works() {
        use crate::auth::Auth;
        let auth: Box<dyn Auth> = Box::new(crate::auth::bearer("token123".into()));
        let mut req = crate::auth::Request {
            method: "GET".into(),
            url: "http://test".into(),
            headers: std::collections::HashMap::new(),
            body: None,
        };
        let result = auth.apply(&mut req).await;
        assert!(result.is_ok());
        assert_eq!(req.headers.get("Authorization").unwrap(), "Bearer token123");
    }
}
