use crate::auth::credentials::{FirebaseToken, RefreshToken};
use anyhow::Result;

#[test]
fn test_firebase_token_urls() -> Result<()> {
    let custom_token = FirebaseToken::Custom("ct".to_string());
    let refresh_token = FirebaseToken::Refresh(RefreshToken::new("rt".to_string()));

    assert!(custom_token.access_token_url("api_key").is_err());
    assert!(refresh_token.access_token_url("api_key").is_err());

    assert_eq!(
        custom_token.access_token_request_body(),
        vec![("returnSecureToken", "true"), ("token", "ct")]
    );
    assert_eq!(
        refresh_token.access_token_request_body(),
        vec![("grant_type", "refresh_token"), ("refresh_token", "rt")],
    );

    assert_eq!(
        custom_token.proxy_url("https://staging.warp.dev", "api_key"),
        "https://staging.warp.dev/proxy/customToken?key=api_key"
    );
    assert_eq!(
        refresh_token.proxy_url("https://staging.warp.dev", "api_key"),
        "https://staging.warp.dev/proxy/token?key=api_key"
    );
    Ok(())
}
