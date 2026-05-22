use std::sync::{Arc, Weak};

use axum::Json;
use axum::http::StatusCode;
use serde_json::json;

use crate::CliResult;
use crate::gateway::control::GatewayControlAppState;
use crate::mvp;

use super::event_bus::{GatewayEventBus, GatewayEventReplayWindow, GatewayEventBusSnapshot};
use super::state::{GatewayPairingRuntimeState, write_gateway_pairing_runtime_state};

type GatewayControlJsonResponse = (StatusCode, Json<serde_json::Value>);

fn json_error(status_code: StatusCode, code: &str, message: &str) -> GatewayControlJsonResponse {
    let payload = json!({
        "error": {
            "code": code,
            "message": message,
        }
    });
    (status_code, Json(payload))
}

pub(super) fn attach_gateway_pairing_runtime_persist_hook(app_state: Arc<GatewayControlAppState>) {
    let Some(event_bus) = app_state.event_bus.as_ref() else {
        return;
    };
    let weak_app_state: Weak<GatewayControlAppState> = Arc::downgrade(&app_state);
    event_bus.set_publish_hook(Arc::new(move || {
        if let Some(app_state) = weak_app_state.upgrade() {
            let _ = persist_gateway_pairing_runtime_state(app_state.as_ref());
        }
    }));
}

pub(super) fn persist_gateway_pairing_runtime_state(app_state: &GatewayControlAppState) -> CliResult<()> {
    let sessions = app_state.connection_registry.snapshot_leases();
    let max_acknowledged_seq = sessions
        .iter()
        .filter_map(|lease| lease.acknowledged_seq)
        .max()
        .unwrap_or(0);
    let event_bus_snapshot = app_state
        .event_bus
        .as_ref()
        .map(GatewayEventBus::snapshot)
        .unwrap_or(GatewayEventBusSnapshot {
            next_seq: 0,
            recent_events: Vec::new(),
        });
    let event_bus = GatewayEventBusSnapshot {
        next_seq: event_bus_snapshot.next_seq.max(max_acknowledged_seq),
        recent_events: event_bus_snapshot.recent_events,
    };
    let state = GatewayPairingRuntimeState {
        sessions,
        event_bus,
    };
    write_gateway_pairing_runtime_state(app_state.runtime_dir.as_path(), &state)
}

pub(super) fn resolve_gateway_pairing_session_lease(
    app_state: &GatewayControlAppState,
    token: &str,
) -> Result<mvp::control_plane::ControlPlaneConnectionLease, GatewayControlJsonResponse> {
    let lease = app_state
        .connection_registry
        .resolve(token)
        .map_err(|error| {
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "session_registry_failed",
                error.as_str(),
            )
        })?;
    let Some(lease) = lease else {
        return Err(json_error(
            StatusCode::UNAUTHORIZED,
            "invalid_session_token",
            "invalid or expired gateway pairing session token",
        ));
    };
    Ok(lease)
}

pub(super) fn ensure_gateway_pairing_session_scope(
    lease: &mvp::control_plane::ControlPlaneConnectionLease,
    required_scope: loong_protocol::ControlPlaneScope,
) -> Result<(), GatewayControlJsonResponse> {
    let has_required_scope = lease.principal.scopes.iter().any(|scope| {
        scope == required_scope.as_str()
            || scope == loong_protocol::ControlPlaneScope::OperatorAdmin.as_str()
    });
    if !has_required_scope {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "insufficient_scope",
            "gateway pairing session token does not grant the required scope",
        ));
    }
    Ok(())
}

pub(super) fn gateway_pairing_after_seq_is_stale(
    after_seq: u64,
    replay_window: GatewayEventReplayWindow,
) -> bool {
    let Some(oldest_retained_seq) = replay_window.oldest_retained_seq else {
        return false;
    };
    after_seq < oldest_retained_seq.saturating_sub(1)
}

pub(super) fn gateway_pairing_event_bus(
    app_state: &GatewayControlAppState,
) -> Result<&GatewayEventBus, GatewayControlJsonResponse> {
    app_state.event_bus.as_ref().ok_or_else(|| {
        json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "event_stream_unavailable",
            "gateway event streaming is not available",
        )
    })
}

pub(super) fn gateway_pairing_stale_cursor_response(
    after_seq: u64,
    last_acknowledged_seq: Option<u64>,
    replay_window: GatewayEventReplayWindow,
) -> GatewayControlJsonResponse {
    let message = match (replay_window.oldest_retained_seq, replay_window.latest_seq) {
        (Some(oldest), Some(latest)) => format!(
            "requested after_seq={} is older than retained replay window {}..{}",
            after_seq, oldest, latest
        ),
        _ => format!("requested after_seq={after_seq} is outside the retained replay window"),
    };
    json_stale_cursor_error(message.as_str(), last_acknowledged_seq, replay_window)
}

fn json_stale_cursor_error(
    message: &str,
    last_acknowledged_seq: Option<u64>,
    replay_window: GatewayEventReplayWindow,
) -> GatewayControlJsonResponse {
    let earliest_resumable_after_seq = replay_window
        .oldest_retained_seq
        .map(|seq| seq.saturating_sub(1))
        .unwrap_or(0);
    let payload = json!({
        "error": {
            "code": "stale_cursor",
            "message": message,
            "last_acknowledged_seq": last_acknowledged_seq,
            "earliest_resumable_after_seq": earliest_resumable_after_seq,
            "replay_window": {
                "oldest_retained_seq": replay_window.oldest_retained_seq,
                "latest_seq": replay_window.latest_seq,
            }
        }
    });
    (StatusCode::CONFLICT, Json(payload))
}
