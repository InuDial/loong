use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use loong_protocol::ControlPlaneConnectRequest;

pub(crate) fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub(crate) fn control_plane_device_signature_message(
    request: &ControlPlaneConnectRequest,
    device: &loong_protocol::ControlPlaneDeviceIdentity,
) -> Vec<u8> {
    let scopes = request
        .scopes
        .iter()
        .map(|scope| scope.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "loong-control-plane-connect-v1\nnonce={}\ndevice_id={}\nclient_id={}\nrole={}\nscopes={}\nsigned_at_ms={}",
        device.nonce,
        device.device_id,
        request.client.id,
        request.role.as_str(),
        scopes,
        device.signed_at_ms
    )
    .into_bytes()
}

pub(crate) fn validate_control_plane_device_challenge(
    request: &ControlPlaneConnectRequest,
    challenge: &crate::mvp::control_plane::ControlPlaneChallenge,
    future_skew_ms: u64,
    now_ms: u64,
) -> Result<(), String> {
    let Some(device) = request.device.as_ref() else {
        return Ok(());
    };

    if device.signed_at_ms < challenge.issued_at_ms
        || device.signed_at_ms > challenge.expires_at_ms.saturating_add(future_skew_ms)
        || device.signed_at_ms > now_ms.saturating_add(future_skew_ms)
    {
        return Err(format!(
            "control-plane device signature timestamp is outside the challenge window for `{}`",
            device.device_id
        ));
    }

    let public_key_bytes = base64::engine::general_purpose::STANDARD
        .decode(device.public_key.as_bytes())
        .map_err(|error| format!("invalid control-plane device public_key encoding: {error}"))?;
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(device.signature.as_bytes())
        .map_err(|error| format!("invalid control-plane device signature encoding: {error}"))?;

    let public_key_array: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| "control-plane device public_key must decode to 32 bytes".to_owned())?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|error| format!("invalid control-plane device public_key: {error}"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| format!("invalid control-plane device signature bytes: {error}"))?;
    let message = control_plane_device_signature_message(request, device);
    verifying_key
        .verify(&message, &signature)
        .map_err(|error| format!("control-plane device signature verification failed: {error}"))?;

    Ok(())
}
