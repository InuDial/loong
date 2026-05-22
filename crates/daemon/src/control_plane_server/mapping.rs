use super::*;

pub(super) fn default_policy() -> ControlPlanePolicy {
    ControlPlanePolicy {
        max_payload_bytes: CONTROL_PLANE_MAX_PAYLOAD_BYTES,
        max_buffered_bytes: CONTROL_PLANE_MAX_BUFFERED_BYTES,
        tick_interval_ms: CONTROL_PLANE_TICK_INTERVAL_MS,
    }
}

pub(super) fn map_snapshot(
    snapshot: mvp::control_plane::ControlPlaneSnapshotSummary,
) -> ControlPlaneSnapshot {
    ControlPlaneSnapshot {
        state_version: ControlPlaneStateVersion {
            presence: snapshot.state_version.presence,
            health: snapshot.state_version.health,
            sessions: snapshot.state_version.sessions,
            approvals: snapshot.state_version.approvals,
            acp: snapshot.state_version.acp,
        },
        presence_count: snapshot.presence_count,
        session_count: snapshot.session_count,
        pending_approval_count: snapshot.pending_approval_count,
        acp_session_count: snapshot.acp_session_count,
        runtime_ready: snapshot.runtime_ready,
    }
}

pub(super) fn map_event_name(
    kind: mvp::control_plane::ControlPlaneEventKind,
) -> ControlPlaneEventName {
    match kind {
        mvp::control_plane::ControlPlaneEventKind::PresenceChanged => {
            ControlPlaneEventName::PresenceChanged
        }
        mvp::control_plane::ControlPlaneEventKind::HealthChanged => {
            ControlPlaneEventName::HealthChanged
        }
        mvp::control_plane::ControlPlaneEventKind::SessionChanged => {
            ControlPlaneEventName::SessionChanged
        }
        mvp::control_plane::ControlPlaneEventKind::SessionMessage => {
            ControlPlaneEventName::SessionMessage
        }
        mvp::control_plane::ControlPlaneEventKind::ApprovalRequested => {
            ControlPlaneEventName::ApprovalRequested
        }
        mvp::control_plane::ControlPlaneEventKind::ApprovalResolved => {
            ControlPlaneEventName::ApprovalResolved
        }
        mvp::control_plane::ControlPlaneEventKind::PairingRequested => {
            ControlPlaneEventName::PairingRequested
        }
        mvp::control_plane::ControlPlaneEventKind::PairingResolved => {
            ControlPlaneEventName::PairingResolved
        }
        mvp::control_plane::ControlPlaneEventKind::AcpSessionChanged => {
            ControlPlaneEventName::AcpSessionChanged
        }
        mvp::control_plane::ControlPlaneEventKind::AcpTurnEvent => {
            ControlPlaneEventName::AcpTurnEvent
        }
    }
}

pub(super) fn map_event(
    event: mvp::control_plane::ControlPlaneEventRecord,
) -> ControlPlaneEventEnvelope {
    ControlPlaneEventEnvelope {
        event: map_event_name(event.kind),
        seq: event.seq,
        state_version: Some(ControlPlaneStateVersion {
            presence: event.state_version.presence,
            health: event.state_version.health,
            sessions: event.state_version.sessions,
            approvals: event.state_version.approvals,
            acp: event.state_version.acp,
        }),
        payload: event.payload,
    }
}

pub(super) fn map_pairing_request(
    request: mvp::control_plane::ControlPlanePairingRequestRecord,
) -> ControlPlanePairingRequestSummary {
    crate::pairing_projection::map_pairing_request_summary(request)
}

pub(super) fn principal_from_connect(
    request: &ControlPlaneConnectRequest,
    connection_id: String,
    granted_scopes: std::collections::BTreeSet<ControlPlaneScope>,
) -> ControlPlanePrincipal {
    crate::control_plane_device_auth::protocol_principal_from_connect_request(
        request,
        connection_id,
        granted_scopes,
    )
}

pub(super) fn parse_pairing_status(
    raw: &str,
) -> Result<mvp::control_plane::ControlPlanePairingStatus, String> {
    crate::pairing_projection::parse_pairing_status(raw)
}

pub(super) fn normalize_required_text(value: &str, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} is required"));
    }
    Ok(trimmed.to_owned())
}

pub(super) fn require_nonempty_text(value: &str, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} is required"));
    }
    Ok(value.to_owned())
}
