
pub mod mock;

pub struct Auth {
    url: String,
    csrf_token: String,
    nonce: String
}

impl Auth {
    pub fn new(url: &str, csrf_token: &str, nonce: &str) -> Self {
        Self {
            url: url.into(),
            csrf_token: csrf_token.into(),
            nonce: nonce.into()
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn csrf_token(&self) -> &str {
        &self.csrf_token
    }

    pub fn nonce(&self) -> &str {
        &self.nonce
    }
}

/// Abstraction over "how do we authenticate a user with an external
/// identity provider." Auth0 is the real implementation; MockProvider
/// is a test double with no network dependency.
#[trait_variant::make(Send)]
pub trait IdentityProvider: Sync {
    /// Build the URL to redirect the user to, plus the CSRF token and
    /// nonce that must be stashed (e.g. in the session) for verification
    /// when the callback arrives.
    fn authorize_url(&self) -> Auth; // (url, csrf_token, nonce)

    /// Exchange an authorization code for verified claims.
    async fn exchange_code(
        &self,
        code: String,
        nonce: String,
    ) -> IDResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedClaims {
    pub sub: String,
    pub email: String,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("token exchange failed: {0}")]
    Exchange(String),
    #[error("claim verification failed: {0}")]
    Verification(String),
}

pub type IDResult = Result<VerifiedClaims, ProviderError>;
