use crate::providers::{Auth, IDResult, IdentityProvider, VerifiedClaims};

/// Test double. Returns a fixed, configurable identity without touching
/// the network. Used in integration tests so the auth flow (redirect,
/// callback, session establishment) can be exercised end-to-end without
/// a real Auth0 tenant.
#[derive(Clone)]
pub struct MockProvider {
    pub claims: VerifiedClaims,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            claims: VerifiedClaims {
                sub: "mock-user-123".into(),
                email: "mock@example.com".into(),
                name: "Mock User".into(),
            },
        }
    }
}

impl IdentityProvider for MockProvider {
    fn authorize_url(&self) -> Auth {
        Auth::new(
            "https://mock-provider.test/authorize?state=fake-state".into(),
            "fake-csrf-token".into(),
            "fake-nonce".into(),
        )
    }

    async fn exchange_code(
        &self,
        _code: String,
        _nonce: String,
    ) -> IDResult {
        Ok(self.claims.clone())
    }
}
