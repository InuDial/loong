use super::*;

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_acp_binding_scope(
    binding: mvp::acp::AcpSessionBindingScope,
) -> ControlPlaneAcpBindingScope {
    ControlPlaneAcpBindingScope {
        route_session_id: binding.route_session_id,
        channel_id: binding.channel_id,
        account_id: binding.account_id,
        conversation_id: binding.conversation_id,
        participant_id: binding.participant_id,
        thread_id: binding.thread_id,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_acp_routing_origin(
    origin: mvp::acp::AcpRoutingOrigin,
) -> ControlPlaneAcpRoutingOrigin {
    match origin {
        mvp::acp::AcpRoutingOrigin::ExplicitRequest => {
            ControlPlaneAcpRoutingOrigin::ExplicitRequest
        }
        mvp::acp::AcpRoutingOrigin::AutomaticAgentPrefixed => {
            ControlPlaneAcpRoutingOrigin::AutomaticAgentPrefixed
        }
        mvp::acp::AcpRoutingOrigin::AutomaticDispatch => {
            ControlPlaneAcpRoutingOrigin::AutomaticDispatch
        }
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_acp_session_mode(mode: mvp::acp::AcpSessionMode) -> ControlPlaneAcpSessionMode {
    match mode {
        mvp::acp::AcpSessionMode::Interactive => ControlPlaneAcpSessionMode::Interactive,
        mvp::acp::AcpSessionMode::Background => ControlPlaneAcpSessionMode::Background,
        mvp::acp::AcpSessionMode::Review => ControlPlaneAcpSessionMode::Review,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_acp_session_state(
    state: mvp::acp::AcpSessionState,
) -> ControlPlaneAcpSessionState {
    match state {
        mvp::acp::AcpSessionState::Initializing => ControlPlaneAcpSessionState::Initializing,
        mvp::acp::AcpSessionState::Ready => ControlPlaneAcpSessionState::Ready,
        mvp::acp::AcpSessionState::Busy => ControlPlaneAcpSessionState::Busy,
        mvp::acp::AcpSessionState::Cancelling => ControlPlaneAcpSessionState::Cancelling,
        mvp::acp::AcpSessionState::Error => ControlPlaneAcpSessionState::Error,
        mvp::acp::AcpSessionState::Closed => ControlPlaneAcpSessionState::Closed,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_acp_session_metadata(
    metadata: mvp::acp::AcpSessionMetadata,
) -> ControlPlaneAcpSessionMetadata {
    ControlPlaneAcpSessionMetadata {
        session_key: metadata.session_key,
        conversation_id: metadata.conversation_id,
        binding: metadata.binding.map(map_acp_binding_scope),
        activation_origin: metadata.activation_origin.map(map_acp_routing_origin),
        backend_id: metadata.backend_id,
        runtime_session_name: metadata.runtime_session_name,
        working_directory: metadata
            .working_directory
            .map(|path| path.display().to_string()),
        backend_session_id: metadata.backend_session_id,
        agent_session_id: metadata.agent_session_id,
        mode: metadata.mode.map(map_acp_session_mode),
        state: map_acp_session_state(metadata.state),
        last_activity_ms: metadata.last_activity_ms,
        last_error: metadata.last_error,
    }
}

#[cfg(feature = "memory-sqlite")]
pub(super) fn map_acp_session_status(
    status: mvp::acp::AcpSessionStatus,
) -> ControlPlaneAcpSessionStatus {
    ControlPlaneAcpSessionStatus {
        session_key: status.session_key,
        backend_id: status.backend_id,
        conversation_id: status.conversation_id,
        binding: status.binding.map(map_acp_binding_scope),
        activation_origin: status.activation_origin.map(map_acp_routing_origin),
        state: map_acp_session_state(status.state),
        mode: status.mode.map(map_acp_session_mode),
        pending_turns: status.pending_turns,
        active_turn_id: status.active_turn_id,
        last_activity_ms: status.last_activity_ms,
        last_error: status.last_error,
    }
}
