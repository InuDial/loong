use super::*;

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_kind(
    kind: mvp::session::repository::SessionKind,
) -> ControlPlaneSessionKind {
    match kind {
        mvp::session::repository::SessionKind::Root => ControlPlaneSessionKind::Root,
        mvp::session::repository::SessionKind::DelegateChild => {
            ControlPlaneSessionKind::DelegateChild
        }
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_state(
    state: mvp::session::repository::SessionState,
) -> ControlPlaneSessionState {
    match state {
        mvp::session::repository::SessionState::Ready => ControlPlaneSessionState::Ready,
        mvp::session::repository::SessionState::Running => ControlPlaneSessionState::Running,
        mvp::session::repository::SessionState::Completed => ControlPlaneSessionState::Completed,
        mvp::session::repository::SessionState::Failed => ControlPlaneSessionState::Failed,
        mvp::session::repository::SessionState::TimedOut => ControlPlaneSessionState::TimedOut,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_workflow_continuity(
    continuity: mvp::control_plane::ControlPlaneSessionWorkflowContinuityView,
) -> ControlPlaneSessionWorkflowContinuity {
    ControlPlaneSessionWorkflowContinuity {
        present: continuity.present,
        resolved_identity_present: continuity.resolved_identity_present,
        session_profile_projection_present: continuity.session_profile_projection_present,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_workflow(
    workflow: mvp::control_plane::ControlPlaneSessionWorkflowView,
) -> ControlPlaneSessionWorkflow {
    let runtime_self_continuity = workflow
        .runtime_self_continuity
        .map(map_session_workflow_continuity);
    let binding = workflow.binding.map(map_session_workflow_binding);

    ControlPlaneSessionWorkflow {
        workflow_id: workflow.workflow_id,
        task: workflow.task,
        phase: workflow.phase,
        operation_kind: workflow.operation_kind,
        operation_scope: workflow.operation_scope,
        task_session_id: workflow.task_session_id,
        lineage_root_session_id: workflow.lineage_root_session_id,
        lineage_depth: workflow.lineage_depth,
        runtime_self_continuity,
        binding,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_workflow_binding(
    binding: mvp::control_plane::ControlPlaneSessionWorkflowBindingView,
) -> ControlPlaneSessionWorkflowBinding {
    let worktree = binding
        .worktree
        .map(|worktree| ControlPlaneSessionWorkflowBindingWorktree {
            worktree_id: worktree.worktree_id,
            workspace_root: worktree.workspace_root,
        });

    ControlPlaneSessionWorkflowBinding {
        session_id: binding.session_id,
        task_id: binding.task_id,
        task_session_id: Some(binding.task_session_id),
        mode: binding.mode,
        execution_surface: binding.execution_surface,
        worktree,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_summary(
    summary: mvp::control_plane::ControlPlaneSessionSummaryView,
) -> ControlPlaneSessionSummary {
    let session = summary.session;
    let workflow = map_session_workflow(summary.workflow);

    ControlPlaneSessionSummary {
        session_id: session.session_id,
        kind: map_session_kind(session.kind),
        parent_session_id: session.parent_session_id,
        label: session.label,
        state: map_session_state(session.state),
        created_at: session.created_at,
        updated_at: session.updated_at,
        archived_at: session.archived_at,
        turn_count: session.turn_count,
        last_turn_at: session.last_turn_at,
        last_error: session.last_error,
        workflow,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_event(
    event: mvp::session::repository::SessionEventRecord,
) -> ControlPlaneSessionEvent {
    ControlPlaneSessionEvent {
        id: event.id,
        session_id: event.session_id,
        event_kind: event.event_kind,
        actor_session_id: event.actor_session_id,
        payload: event.payload_json,
        ts: event.ts,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_terminal_outcome(
    outcome: mvp::session::repository::SessionTerminalOutcomeRecord,
) -> ControlPlaneSessionTerminalOutcome {
    ControlPlaneSessionTerminalOutcome {
        session_id: outcome.session_id,
        status: outcome.status,
        payload: outcome.payload_json,
        recorded_at: outcome.recorded_at,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_session_observation(
    observation: mvp::control_plane::ControlPlaneSessionObservationView,
) -> ControlPlaneSessionObservation {
    ControlPlaneSessionObservation {
        session: map_session_summary(observation.session),
        terminal_outcome: observation
            .terminal_outcome
            .map(map_session_terminal_outcome),
        recent_events: observation
            .recent_events
            .into_iter()
            .map(map_session_event)
            .collect::<Vec<_>>(),
        tail_events: observation
            .tail_events
            .into_iter()
            .map(map_session_event)
            .collect::<Vec<_>>(),
    }
}
