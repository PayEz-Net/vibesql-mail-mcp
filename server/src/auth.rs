use actix_web::dev::ServiceRequest;
use actix_web::error::ErrorUnauthorized;
use actix_web::Error;

/// Simple shared secret auth. Checks X-Mail-Secret header against configured secret.
/// Dev mode bypasses all auth.
pub fn check_auth(req: &ServiceRequest, secret: &Option<String>, dev_mode: bool) -> Result<(), Error> {
    if dev_mode {
        return Ok(());
    }

    let configured = match secret {
        Some(s) if !s.is_empty() => s,
        _ => return Err(ErrorUnauthorized("No secret configured and dev mode is off")),
    };

    let provided = req
        .headers()
        .get("X-Mail-Secret")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ErrorUnauthorized("Missing X-Mail-Secret header"))?;

    if provided != configured {
        return Err(ErrorUnauthorized("Invalid secret"));
    }

    Ok(())
}
