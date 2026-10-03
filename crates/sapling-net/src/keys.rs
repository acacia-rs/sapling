use acacia_auth::login::verify::SigningKeys;
use acacia_auth::{AuthClient, AuthConfig};

#[derive(Debug, thiserror::Error)]
#[error("fetching the login signing keys: {0}")]
pub struct KeyError(#[from] acacia_auth::Error);

/// The authorization service's current signing keys, which online-mode logins are verified against.
pub async fn fetch_signing_keys() -> Result<SigningKeys, KeyError> {
    Ok(AuthClient::new(AuthConfig::default())?.fetch_signing_keys().await?)
}
