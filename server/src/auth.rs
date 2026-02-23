use actix_web::dev::ServiceRequest;
use actix_web::error::ErrorUnauthorized;
use actix_web::Error;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const MAX_TIMESTAMP_DRIFT_SECS: u64 = 300; // 5 minutes

pub fn verify_hmac(req: &ServiceRequest, secret: &str) -> Result<(), Error> {
    let timestamp = req
        .headers()
        .get("X-Vibe-Timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ErrorUnauthorized("Missing X-Vibe-Timestamp header"))?;

    let signature = req
        .headers()
        .get("X-Vibe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ErrorUnauthorized("Missing X-Vibe-Signature header"))?;

    // Verify timestamp is within 5 minutes
    let ts: u64 = timestamp
        .parse()
        .map_err(|_| ErrorUnauthorized("Invalid timestamp"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let drift = if now > ts { now - ts } else { ts - now };
    if drift > MAX_TIMESTAMP_DRIFT_SECS {
        return Err(ErrorUnauthorized("Timestamp expired"));
    }

    // Build signing string: {timestamp}|{method}|{path}
    let method = req.method().as_str();
    let path = req.path();
    let message = format!("{}|{}|{}", timestamp, method, path);

    // Compute HMAC
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| ErrorUnauthorized("HMAC error"))?;
    mac.update(message.as_bytes());
    let expected = mac.finalize().into_bytes();
    let expected_b64 = base64::engine::general_purpose::STANDARD.encode(expected);

    if signature != expected_b64 {
        return Err(ErrorUnauthorized("Invalid signature"));
    }

    Ok(())
}

/// Middleware-style auth check. Returns Ok(()) if dev mode or valid HMAC.
pub fn check_auth(req: &ServiceRequest, secret: &Option<String>, dev_mode: bool) -> Result<(), Error> {
    if dev_mode {
        return Ok(());
    }

    match secret {
        Some(s) => verify_hmac(req, s),
        None => Err(ErrorUnauthorized("No secret configured and dev mode is off")),
    }
}
