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

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_task_summary(
    task: mvp::control_plane::ControlPlaneTaskSummaryView,
) -> ControlPlaneTaskSummary {
    let workflow = map_session_workflow(task.workflow);
    let session_state = task.session_state;
    let delegate_phase = task.delegate_phase;
    let delegate_mode = task.delegate_mode;
    let timeout_seconds = task.timeout_seconds;
    let approval_request_count = task.approval_request_count;
    let approval_attention_count = task.approval_attention_count;
    let requested_tool_ids = task.requested_tool_ids;
    let visible_requested_tool_ids = task.visible_requested_tool_ids;
    let effective_tool_ids = task.effective_tool_ids;
    let visible_effective_tool_ids = task.visible_effective_tool_ids;
    let effective_runtime_narrowing = task.effective_runtime_narrowing;
    let label = task.label;
    let last_error = task.last_error;

    ControlPlaneTaskSummary {
        task_id: task.task_id,
        task_session_id: task.task_session_id,
        owner_session_id: task.owner_session_id,
        session_id: task.session_id,
        scope_session_id: task.scope_session_id,
        label,
        session_state,
        delegate_phase,
        delegate_mode,
        timeout_seconds,
        workflow,
        approval_request_count,
        approval_attention_count,
        requested_tool_ids,
        visible_requested_tool_ids,
        effective_tool_ids,
        visible_effective_tool_ids,
        effective_runtime_narrowing,
        last_error,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_approval_status(
    status: mvp::session::repository::ApprovalRequestStatus,
) -> ControlPlaneApprovalRequestStatus {
    match status {
        mvp::session::repository::ApprovalRequestStatus::Pending => {
            ControlPlaneApprovalRequestStatus::Pending
        }
        mvp::session::repository::ApprovalRequestStatus::Approved => {
            ControlPlaneApprovalRequestStatus::Approved
        }
        mvp::session::repository::ApprovalRequestStatus::Executing => {
            ControlPlaneApprovalRequestStatus::Executing
        }
        mvp::session::repository::ApprovalRequestStatus::Executed => {
            ControlPlaneApprovalRequestStatus::Executed
        }
        mvp::session::repository::ApprovalRequestStatus::Denied => {
            ControlPlaneApprovalRequestStatus::Denied
        }
        mvp::session::repository::ApprovalRequestStatus::Expired => {
            ControlPlaneApprovalRequestStatus::Expired
        }
        mvp::session::repository::ApprovalRequestStatus::Cancelled => {
            ControlPlaneApprovalRequestStatus::Cancelled
        }
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_approval_decision(
    decision: mvp::session::repository::ApprovalDecision,
) -> ControlPlaneApprovalDecision {
    match decision {
        mvp::session::repository::ApprovalDecision::ApproveOnce => {
            ControlPlaneApprovalDecision::ApproveOnce
        }
        mvp::session::repository::ApprovalDecision::ApproveAlways => {
            ControlPlaneApprovalDecision::ApproveAlways
        }
        mvp::session::repository::ApprovalDecision::Deny => ControlPlaneApprovalDecision::Deny,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_approval_summary(
    approval: mvp::session::repository::ApprovalRequestRecord,
) -> ControlPlaneApprovalSummary {
    let reason = approval
        .governance_snapshot_json
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    let rule_id = approval
        .governance_snapshot_json
        .get("rule_id")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    let visible_tool_name = Some(mvp::tools::user_visible_tool_name(
        approval.tool_name.as_str(),
    ));
    let raw_request = approval
        .request_payload_json
        .as_object()
        .and_then(|payload| payload.get("args_json"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let summarized_request =
        mvp::tools::summarize_tool_request_for_display(approval.tool_name.as_str(), raw_request);
    let request_summary = Some(serde_json::json!({
        "tool": visible_tool_name.clone().unwrap_or_else(|| approval.tool_name.clone()),
        "request": summarized_request,
    }));
    ControlPlaneApprovalSummary {
        approval_request_id: approval.approval_request_id,
        session_id: approval.session_id,
        turn_id: approval.turn_id,
        tool_call_id: approval.tool_call_id,
        tool_name: approval.tool_name,
        visible_tool_name,
        request_summary,
        approval_key: approval.approval_key,
        status: map_approval_status(approval.status),
        decision: approval.decision.map(map_approval_decision),
        requested_at: approval.requested_at,
        resolved_at: approval.resolved_at,
        resolved_by_session_id: approval.resolved_by_session_id,
        executed_at: approval.executed_at,
        last_error: approval.last_error,
        reason,
        rule_id,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn parse_approval_request_status(
    raw: &str,
) -> Result<mvp::session::repository::ApprovalRequestStatus, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "pending" => Ok(mvp::session::repository::ApprovalRequestStatus::Pending),
        "approved" => Ok(mvp::session::repository::ApprovalRequestStatus::Approved),
        "executing" => Ok(mvp::session::repository::ApprovalRequestStatus::Executing),
        "executed" => Ok(mvp::session::repository::ApprovalRequestStatus::Executed),
        "denied" => Ok(mvp::session::repository::ApprovalRequestStatus::Denied),
        "expired" => Ok(mvp::session::repository::ApprovalRequestStatus::Expired),
        "cancelled" => Ok(mvp::session::repository::ApprovalRequestStatus::Cancelled),
        _ => Err(format!("unknown approval status `{raw}`")),
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
