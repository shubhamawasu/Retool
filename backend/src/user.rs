use axum_login::AuthUser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
}

impl AuthUser for User {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.sub.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.email.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_login::AuthUser;

    fn sample_user() -> User {
        User {
            sub: "user-123".into(),
            email: "test@example.com".into(),
            name: Some("Test User".into()),
        }
    }

    #[test]
    fn id_returns_the_users_id() {
        let user = sample_user();
        assert_eq!(user.id(), "user-123");
    }

    #[test]
    fn session_auth_hash_is_deterministic() {
        let user = sample_user();
        let user2 = sample_user();
        assert_eq!(user.session_auth_hash(), user2.session_auth_hash());
    }

    #[test]
    fn session_auth_hash_differs_for_different_emails() {
        let user_a = User {
            sub: "user-1".into(),
            email: "a@example.com".into(),
            name: None,
        };
        let user_b = User {
            sub: "user-2".into(),
            email: "b@example.com".into(),
            name: None,
        };
        assert_ne!(user_a.session_auth_hash(), user_b.session_auth_hash());
    }
}
