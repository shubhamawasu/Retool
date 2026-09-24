use crate::user::User;
use axum_login::AuthnBackend;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Default)]
pub struct MemoryBackend {
    users: Arc<RwLock<HashMap<String, User>>>,
}

/// Placeholder for whatever "proof of login" gets passed to `authenticate`.
/// For now this is just a hardcoded stand-in; later this becomes the
/// verified claims from Auth0.
#[derive(Debug, Clone)]
pub struct OidcCredentials {
    pub sub: String,
    pub email: String,
    pub name: String,
}

impl AuthnBackend for MemoryBackend {
    type User = User;
    type Credentials = OidcCredentials;
    type Error = std::convert::Infallible;
    
    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = User {
            sub: creds.sub.clone(),
            email: creds.email,
            name: None,
        };
        self.users.write().unwrap().insert(creds.sub, user.clone());
        Ok(Some(user))
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        Ok(self.users.read().unwrap().get(user_id).cloned())
    }
}

pub type AuthSession = axum_login::AuthSession<MemoryBackend>;

#[cfg(test)]
mod tests {
    use super::*;

    fn creds(id: &str, email: &str, name: &str) -> OidcCredentials {
        OidcCredentials {
            sub: id.into(),
            email: email.into(),
            name: name.into(),
        }
    }

    #[tokio::test]
    async fn authenticate_returns_user_with_matching_fields() {
        let backend = MemoryBackend::default();

        let user = backend
            .authenticate(creds("abc", "a@example.com", "Bob"))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(user.sub, "abc");
        assert_eq!(user.email, "a@example.com");
    }

    #[tokio::test]
    async fn authenticate_then_get_user_roundtrips() {
        let backend = MemoryBackend::default();
        backend.authenticate(creds("abc", "a@example.com", "Bob")).await.unwrap();

        let fetched = backend.get_user(&"abc".to_string()).await.unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().email, "a@example.com");
    }

    #[tokio::test]
    async fn get_user_returns_none_for_unknown_id() {
        let backend = MemoryBackend::default();

        let fetched = backend.get_user(&"nonexistent".to_string()).await.unwrap();

        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn authenticate_upserts_existing_user() {
        let backend = MemoryBackend::default();
        backend.authenticate(creds("abc", "old@example.com", "Bob")).await.unwrap();
        backend.authenticate(creds("abc", "new@example.com", "Bob")).await.unwrap();

        let fetched = backend.get_user(&"abc".to_string()).await.unwrap().unwrap();

        assert_eq!(fetched.email, "new@example.com");
    }

    #[tokio::test]
    async fn authenticate_does_not_affect_other_users() {
        let backend = MemoryBackend::default();
        backend.authenticate(creds("abc", "a@example.com", "Bob")).await.unwrap();
        backend.authenticate(creds("xyz", "x@example.com", "Bob")).await.unwrap();

        let a = backend.get_user(&"abc".to_string()).await.unwrap().unwrap();
        let x = backend.get_user(&"xyz".to_string()).await.unwrap().unwrap();

        assert_eq!(a.email, "a@example.com");
        assert_eq!(x.email, "x@example.com");
    }

    #[tokio::test]
    async fn concurrent_authenticates_do_not_lose_writes() {
        let backend = MemoryBackend::default();

        let handles: Vec<_> = (0..20)
            .map(|i| {
                let backend = backend.clone();
                tokio::spawn(async move {
                    let id = format!("user-{i}");
                    let email = format!("user{i}@example.com");
                    let name = format!("User {i}");
                    backend.authenticate(creds(&id, &email, &name)).await.unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        for i in 0..20 {
            let id = format!("user-{i}");
            let expected_email = format!("user{i}@example.com");
            let fetched = backend.get_user(&id).await.unwrap();
            assert_eq!(fetched.unwrap().email, expected_email);
        }
    }
}
