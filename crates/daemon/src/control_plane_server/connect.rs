use super::*;

pub(super) fn pairing_required_response(
    request: &mvp::control_plane::ControlPlanePairingRequestRecord,
) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ControlPlaneConnectErrorResponse {
            code: ControlPlaneConnectErrorCode::PairingRequired,
            error: format!(
                "device `{}` requires operator pairing approval before connect can complete",
                request.device_id
            ),
            pairing_request_id: Some(request.pairing_request_id.clone()),
        }),
    )
        .into_response()
}

pub(super) fn device_token_error_response(
    code: ControlPlaneConnectErrorCode,
    error: impl Into<String>,
) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ControlPlaneConnectErrorResponse {
            code,
            error: error.into(),
            pairing_request_id: None,
        }),
    )
        .into_response()
}

pub(super) fn connect_error_response(
    status: StatusCode,
    code: ControlPlaneConnectErrorCode,
    error: impl Into<String>,
) -> Response {
    (
        status,
        Json(ControlPlaneConnectErrorResponse {
            code,
            error: error.into(),
            pairing_request_id: None,
        }),
    )
        .into_response()
}

pub(super) fn error_response(status: StatusCode, error: impl Into<String>) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": error.into(),
        })),
    )
        .into_response()
}

pub(super) fn granted_connect_scopes(
    state: &ControlPlaneHttpState,
    request: &ControlPlaneConnectRequest,
) -> std::collections::BTreeSet<ControlPlaneScope> {
    let remote_bootstrap = state.exposure_policy.requires_remote_auth() && request.device.is_none();
    if !remote_bootstrap {
        return request.scopes.clone();
    }

    let allowed_scopes = std::collections::BTreeSet::from(CONTROL_PLANE_REMOTE_BOOTSTRAP_SCOPES);
    let requested_scopes = request.scopes.clone();
    requested_scopes
        .intersection(&allowed_scopes)
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
}
