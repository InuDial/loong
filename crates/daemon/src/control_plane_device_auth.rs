use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use loong_protocol::{ControlPlaneConnectRequest, ControlPlaneScope};

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

    let public_key_array: [u8; 32] = public_key_bytes.try_into().map_err(|bytes: Vec<u8>| {
        format!(
            "control-plane device public_key must decode to 32 bytes, got {} bytes",
            bytes.len()
        )
    })?;
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

pub(crate) fn requested_scope_names(
    request: &ControlPlaneConnectRequest,
) -> std::collections::BTreeSet<String> {
    request
        .scopes
        .iter()
        .map(|scope| scope.as_str().to_owned())
        .collect::<std::collections::BTreeSet<_>>()
}

pub(crate) fn presented_device_token(request: &ControlPlaneConnectRequest) -> Option<&str> {
    request
        .auth
        .as_ref()
        .and_then(|auth| auth.device_token.as_deref())
}

pub(crate) fn connection_principal_from_connect_request(
    request: &ControlPlaneConnectRequest,
    connection_id: String,
    granted_scopes: &std::collections::BTreeSet<ControlPlaneScope>,
) -> crate::mvp::control_plane::ControlPlaneConnectionPrincipal {
    crate::mvp::control_plane::ControlPlaneConnectionPrincipal {
        connection_id,
        client_id: request.client.id.clone(),
        role: request.role.as_str().to_owned(),
        scopes: granted_scopes
            .iter()
            .map(|scope| scope.as_str().to_owned())
            .collect::<std::collections::BTreeSet<_>>(),
        device_id: request
            .device
            .as_ref()
            .map(|device| device.device_id.clone()),
    }
}

pub(crate) fn protocol_principal_from_connection_lease(
    lease: &crate::mvp::control_plane::ControlPlaneConnectionLease,
) -> loong_protocol::ControlPlanePrincipal {
    let role = match lease.principal.role.as_str() {
        "operator" => loong_protocol::ControlPlaneRole::Operator,
        _ => loong_protocol::ControlPlaneRole::Node,
    };
    let scopes = lease
        .principal
        .scopes
        .iter()
        .filter_map(|scope| loong_protocol::ControlPlaneScope::parse(scope.as_str()))
        .collect();
    loong_protocol::ControlPlanePrincipal {
        connection_id: lease.principal.connection_id.clone(),
        client_id: lease.principal.client_id.clone(),
        role,
        scopes,
        device_id: lease.principal.device_id.clone(),
    }
}

pub(crate) fn protocol_principal_from_connect_request(
    request: &ControlPlaneConnectRequest,
    connection_id: String,
    granted_scopes: std::collections::BTreeSet<ControlPlaneScope>,
) -> loong_protocol::ControlPlanePrincipal {
    loong_protocol::ControlPlanePrincipal {
        connection_id,
        client_id: request.client.id.clone(),
        role: request.role,
        scopes: granted_scopes,
        device_id: request
            .device
            .as_ref()
            .map(|device| device.device_id.clone()),
    }
}

pub(crate) enum PairingConnectOutcome {
    Authorized,
    PairingRequired {
        request: Box<crate::mvp::control_plane::ControlPlanePairingRequestRecord>,
        created: bool,
    },
    DeviceTokenRequired,
    DeviceTokenInvalid,
}

pub(crate) fn normalize_pairing_connect_decision(
    decision: crate::mvp::control_plane::ControlPlanePairingConnectDecision,
) -> PairingConnectOutcome {
    match decision {
        crate::mvp::control_plane::ControlPlanePairingConnectDecision::Authorized => {
            PairingConnectOutcome::Authorized
        }
        crate::mvp::control_plane::ControlPlanePairingConnectDecision::PairingRequired {
            request,
            created,
        } => PairingConnectOutcome::PairingRequired { request, created },
        crate::mvp::control_plane::ControlPlanePairingConnectDecision::DeviceTokenRequired => {
            PairingConnectOutcome::DeviceTokenRequired
        }
        crate::mvp::control_plane::ControlPlanePairingConnectDecision::DeviceTokenInvalid => {
            PairingConnectOutcome::DeviceTokenInvalid
        }
    }
}

pub(crate) fn evaluate_pairing_connect_outcome(
    pairing_registry: &crate::mvp::control_plane::ControlPlanePairingRegistry,
    request: &ControlPlaneConnectRequest,
) -> Result<Option<PairingConnectOutcome>, String> {
    let Some(device) = request.device.as_ref() else {
        return Ok(None);
    };

    let requested_scopes = requested_scope_names(request);
    let device_token = presented_device_token(request);
    let decision = pairing_registry.evaluate_connect(
        device.device_id.as_str(),
        request.client.id.as_str(),
        device.public_key.as_str(),
        request.role.as_str(),
        &requested_scopes,
        device_token,
    )?;

    Ok(Some(normalize_pairing_connect_decision(decision)))
}
