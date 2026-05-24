use super::*;

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
