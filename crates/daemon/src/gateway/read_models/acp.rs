use std::collections::BTreeMap;

use serde::Serialize;

use crate::app;

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpBindingScopeReadModel {
    pub route_session_id: String,
    pub channel_id: Option<String>,
    pub account_id: Option<String>,
    pub conversation_id: Option<String>,
    pub participant_id: Option<String>,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpSessionActivationProvenanceReadModel {
    pub surface: &'static str,
    pub activation_origin: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpSessionMetadataReadModel {
    pub session_key: String,
    pub conversation_id: Option<String>,
    pub binding: Option<GatewayAcpBindingScopeReadModel>,
    pub activation_origin: Option<&'static str>,
    pub provenance: GatewayAcpSessionActivationProvenanceReadModel,
    pub backend_id: String,
    pub runtime_session_name: String,
    pub working_directory: Option<String>,
    pub backend_session_id: Option<String>,
    pub agent_session_id: Option<String>,
    pub mode: Option<&'static str>,
    pub state: &'static str,
    pub last_activity_ms: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpSessionListReadModel {
    pub config: String,
    pub matched_count: usize,
    pub returned_count: usize,
    pub sessions: Vec<GatewayAcpSessionMetadataReadModel>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpSessionStatusReadModel {
    pub session_key: String,
    pub backend_id: String,
    pub conversation_id: Option<String>,
    pub binding: Option<GatewayAcpBindingScopeReadModel>,
    pub activation_origin: Option<&'static str>,
    pub provenance: GatewayAcpSessionActivationProvenanceReadModel,
    pub state: &'static str,
    pub mode: Option<&'static str>,
    pub pending_turns: usize,
    pub active_turn_id: Option<String>,
    pub last_activity_ms: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpStatusReadModel {
    pub config: String,
    pub requested_session: Option<String>,
    pub requested_conversation_id: Option<String>,
    pub requested_route_session_id: Option<String>,
    pub resolved_session_key: String,
    pub status: GatewayAcpSessionStatusReadModel,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpActivationAggregateProvenanceReadModel {
    pub surface: &'static str,
    pub activation_origin_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpRuntimeCacheReadModel {
    pub active_sessions: usize,
    pub idle_ttl_ms: u64,
    pub evicted_total: u64,
    pub last_evicted_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpSessionAggregateReadModel {
    pub bound: usize,
    pub unbound: usize,
    pub activation_origin_counts: BTreeMap<String, usize>,
    pub provenance: GatewayAcpActivationAggregateProvenanceReadModel,
    pub backend_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpActorReadModel {
    pub active: usize,
    pub queue_depth: usize,
    pub waiting: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpTurnReadModel {
    pub active: usize,
    pub queue_depth: usize,
    pub completed: u64,
    pub failed: u64,
    pub average_latency_ms: u64,
    pub max_latency_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpObservabilitySnapshotReadModel {
    pub runtime_cache: GatewayAcpRuntimeCacheReadModel,
    pub sessions: GatewayAcpSessionAggregateReadModel,
    pub actors: GatewayAcpActorReadModel,
    pub turns: GatewayAcpTurnReadModel,
    pub errors_by_code: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpObservabilityReadModel {
    pub config: String,
    pub snapshot: GatewayAcpObservabilitySnapshotReadModel,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayConversationAddressReadModel {
    pub session_id: String,
    pub channel_id: Option<String>,
    pub account_id: Option<String>,
    pub conversation_id: Option<String>,
    pub participant_id: Option<String>,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpDispatchPredictionProvenanceReadModel {
    pub surface: &'static str,
    pub automatic_routing_origin: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpDispatchTargetReadModel {
    pub original_session_id: String,
    pub route_session_id: String,
    pub prefixed_agent_id: Option<String>,
    pub channel_id: Option<String>,
    pub account_id: Option<String>,
    pub conversation_id: Option<String>,
    pub participant_id: Option<String>,
    pub thread_id: Option<String>,
    pub channel_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpDispatchDecisionDetailsReadModel {
    pub route_via_acp: bool,
    pub reason: &'static str,
    pub automatic_routing_origin: Option<&'static str>,
    pub provenance: GatewayAcpDispatchPredictionProvenanceReadModel,
    pub target: GatewayAcpDispatchTargetReadModel,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpDispatchDecisionReadModel {
    pub session: String,
    pub decision: GatewayAcpDispatchDecisionDetailsReadModel,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayAcpDispatchReadModel {
    pub config: String,
    pub address: GatewayConversationAddressReadModel,
    pub dispatch: GatewayAcpDispatchDecisionReadModel,
}

pub fn build_acp_session_list_read_model(
    config_path: &str,
    matched_count: usize,
    sessions: &[app::acp::AcpSessionMetadata],
) -> GatewayAcpSessionListReadModel {
    let config = config_path.to_owned();
    let returned_count = sessions.len();
    let sessions = sessions
        .iter()
        .map(build_acp_session_metadata_read_model)
        .collect();

    GatewayAcpSessionListReadModel {
        config,
        matched_count,
        returned_count,
        sessions,
    }
}

pub fn build_acp_status_read_model(
    config_path: &str,
    requested_session: Option<&str>,
    requested_conversation_id: Option<&str>,
    requested_route_session_id: Option<&str>,
    resolved_session_key: &str,
    status: &app::acp::AcpSessionStatus,
) -> GatewayAcpStatusReadModel {
    let config = config_path.to_owned();
    let requested_session = requested_session.map(str::to_owned);
    let requested_conversation_id = requested_conversation_id.map(str::to_owned);
    let requested_route_session_id = requested_route_session_id.map(str::to_owned);
    let resolved_session_key = resolved_session_key.to_owned();
    let status = build_acp_session_status_read_model(status);

    GatewayAcpStatusReadModel {
        config,
        requested_session,
        requested_conversation_id,
        requested_route_session_id,
        resolved_session_key,
        status,
    }
}

pub fn build_acp_observability_read_model(
    config_path: &str,
    snapshot: &app::acp::AcpManagerObservabilitySnapshot,
) -> GatewayAcpObservabilityReadModel {
    let config = config_path.to_owned();
    let snapshot = build_acp_observability_snapshot_read_model(snapshot);

    GatewayAcpObservabilityReadModel { config, snapshot }
}

pub fn build_acp_dispatch_read_model(
    config_path: &str,
    address: &app::conversation::ConversationSessionAddress,
    session_id: &str,
    decision: &app::acp::AcpConversationDispatchDecision,
) -> GatewayAcpDispatchReadModel {
    let config = config_path.to_owned();
    let address = build_conversation_address_read_model(address);
    let dispatch = build_acp_dispatch_decision_read_model(session_id, decision);

    GatewayAcpDispatchReadModel {
        config,
        address,
        dispatch,
    }
}

fn build_acp_binding_scope_read_model(
    binding: &app::acp::AcpSessionBindingScope,
) -> GatewayAcpBindingScopeReadModel {
    let route_session_id = binding.route_session_id.clone();
    let channel_id = binding.channel_id.clone();
    let account_id = binding.account_id.clone();
    let conversation_id = binding.conversation_id.clone();
    let participant_id = binding.participant_id.clone();
    let thread_id = binding.thread_id.clone();

    GatewayAcpBindingScopeReadModel {
        route_session_id,
        channel_id,
        account_id,
        conversation_id,
        participant_id,
        thread_id,
    }
}

fn build_acp_session_activation_provenance_read_model(
    origin: Option<app::acp::AcpRoutingOrigin>,
) -> GatewayAcpSessionActivationProvenanceReadModel {
    let activation_origin = origin.map(app::acp::AcpRoutingOrigin::as_str);
    let surface = "session_activation";

    GatewayAcpSessionActivationProvenanceReadModel {
        surface,
        activation_origin,
    }
}

fn build_acp_session_metadata_read_model(
    metadata: &app::acp::AcpSessionMetadata,
) -> GatewayAcpSessionMetadataReadModel {
    let shared = build_acp_session_projection(
        &metadata.session_key,
        metadata.conversation_id.clone(),
        metadata.binding.as_ref(),
        metadata.activation_origin,
        metadata.mode,
        metadata.state,
        metadata.last_activity_ms,
        metadata.last_error.clone(),
    );

    GatewayAcpSessionMetadataReadModel {
        session_key: shared.session_key,
        conversation_id: shared.conversation_id,
        binding: shared.binding,
        activation_origin: shared.activation_origin,
        provenance: shared.provenance,
        backend_id: metadata.backend_id.clone(),
        runtime_session_name: metadata.runtime_session_name.clone(),
        working_directory: metadata
            .working_directory
            .as_ref()
            .map(|path| path.display().to_string()),
        backend_session_id: metadata.backend_session_id.clone(),
        agent_session_id: metadata.agent_session_id.clone(),
        mode: shared.mode,
        state: shared.state,
        last_activity_ms: shared.last_activity_ms,
        last_error: shared.last_error,
    }
}

fn build_acp_session_status_read_model(
    status: &app::acp::AcpSessionStatus,
) -> GatewayAcpSessionStatusReadModel {
    let shared = build_acp_session_projection(
        &status.session_key,
        status.conversation_id.clone(),
        status.binding.as_ref(),
        status.activation_origin,
        status.mode,
        status.state,
        status.last_activity_ms,
        status.last_error.clone(),
    );

    GatewayAcpSessionStatusReadModel {
        session_key: shared.session_key,
        backend_id: status.backend_id.clone(),
        conversation_id: shared.conversation_id,
        binding: shared.binding,
        activation_origin: shared.activation_origin,
        provenance: shared.provenance,
        state: shared.state,
        mode: shared.mode,
        pending_turns: status.pending_turns,
        active_turn_id: status.active_turn_id.clone(),
        last_activity_ms: shared.last_activity_ms,
        last_error: shared.last_error,
    }
}

struct GatewayAcpSessionProjection {
    session_key: String,
    conversation_id: Option<String>,
    binding: Option<GatewayAcpBindingScopeReadModel>,
    activation_origin: Option<&'static str>,
    provenance: GatewayAcpSessionActivationProvenanceReadModel,
    mode: Option<&'static str>,
    state: &'static str,
    last_activity_ms: u64,
    last_error: Option<String>,
}

fn build_acp_session_projection(
    session_key: &str,
    conversation_id: Option<String>,
    binding: Option<&app::acp::AcpSessionBindingScope>,
    activation_origin: Option<app::acp::AcpRoutingOrigin>,
    mode: Option<app::acp::AcpSessionMode>,
    state: app::acp::AcpSessionState,
    last_activity_ms: u64,
    last_error: Option<String>,
) -> GatewayAcpSessionProjection {
    GatewayAcpSessionProjection {
        session_key: session_key.to_owned(),
        conversation_id,
        binding: binding.map(build_acp_binding_scope_read_model),
        activation_origin: activation_origin.map(app::acp::AcpRoutingOrigin::as_str),
        provenance: build_acp_session_activation_provenance_read_model(activation_origin),
        mode: mode.map(crate::acp_session_mode_label),
        state: crate::acp_session_state_label(state),
        last_activity_ms,
        last_error,
    }
}

fn build_acp_observability_snapshot_read_model(
    snapshot: &app::acp::AcpManagerObservabilitySnapshot,
) -> GatewayAcpObservabilitySnapshotReadModel {
    GatewayAcpObservabilitySnapshotReadModel {
        runtime_cache: build_acp_runtime_cache_read_model(snapshot),
        sessions: build_acp_session_aggregate_read_model(snapshot),
        actors: build_acp_actor_read_model(snapshot),
        turns: build_acp_turn_read_model(snapshot),
        errors_by_code: snapshot.errors_by_code.clone(),
    }
}

fn build_acp_runtime_cache_read_model(
    snapshot: &app::acp::AcpManagerObservabilitySnapshot,
) -> GatewayAcpRuntimeCacheReadModel {
    GatewayAcpRuntimeCacheReadModel {
        active_sessions: snapshot.runtime_cache.active_sessions,
        idle_ttl_ms: snapshot.runtime_cache.idle_ttl_ms,
        evicted_total: snapshot.runtime_cache.evicted_total,
        last_evicted_at_ms: snapshot.runtime_cache.last_evicted_at_ms,
    }
}

fn build_acp_session_aggregate_read_model(
    snapshot: &app::acp::AcpManagerObservabilitySnapshot,
) -> GatewayAcpSessionAggregateReadModel {
    let activation_origin_counts = snapshot.sessions.activation_origin_counts.clone();
    let provenance = GatewayAcpActivationAggregateProvenanceReadModel {
        surface: "session_activation_aggregate",
        activation_origin_counts: activation_origin_counts.clone(),
    };

    GatewayAcpSessionAggregateReadModel {
        bound: snapshot.sessions.bound,
        unbound: snapshot.sessions.unbound,
        activation_origin_counts,
        provenance,
        backend_counts: snapshot.sessions.backend_counts.clone(),
    }
}

fn build_acp_actor_read_model(
    snapshot: &app::acp::AcpManagerObservabilitySnapshot,
) -> GatewayAcpActorReadModel {
    GatewayAcpActorReadModel {
        active: snapshot.actors.active,
        queue_depth: snapshot.actors.queue_depth,
        waiting: snapshot.actors.waiting,
    }
}

fn build_acp_turn_read_model(
    snapshot: &app::acp::AcpManagerObservabilitySnapshot,
) -> GatewayAcpTurnReadModel {
    GatewayAcpTurnReadModel {
        active: snapshot.turns.active,
        queue_depth: snapshot.turns.queue_depth,
        completed: snapshot.turns.completed,
        failed: snapshot.turns.failed,
        average_latency_ms: snapshot.turns.average_latency_ms,
        max_latency_ms: snapshot.turns.max_latency_ms,
    }
}

fn build_conversation_address_read_model(
    address: &app::conversation::ConversationSessionAddress,
) -> GatewayConversationAddressReadModel {
    let route = build_acp_route_projection(
        address.channel_id.clone(),
        address.account_id.clone(),
        address.conversation_id.clone(),
        address.participant_id.clone(),
        address.thread_id.clone(),
    );

    GatewayConversationAddressReadModel {
        session_id: address.session_id.clone(),
        channel_id: route.channel_id,
        account_id: route.account_id,
        conversation_id: route.conversation_id,
        participant_id: route.participant_id,
        thread_id: route.thread_id,
    }
}

fn build_acp_dispatch_prediction_provenance_read_model(
    decision: &app::acp::AcpConversationDispatchDecision,
) -> GatewayAcpDispatchPredictionProvenanceReadModel {
    let surface = "dispatch_prediction";
    let automatic_routing_origin = decision
        .automatic_routing_origin
        .map(app::acp::AcpRoutingOrigin::as_str);

    GatewayAcpDispatchPredictionProvenanceReadModel {
        surface,
        automatic_routing_origin,
    }
}

fn build_acp_dispatch_target_read_model(
    target: &app::acp::AcpConversationDispatchTarget,
) -> GatewayAcpDispatchTargetReadModel {
    let route = build_acp_route_projection(
        target.channel_id.clone(),
        target.account_id.clone(),
        target.conversation_id.clone(),
        target.participant_id.clone(),
        target.thread_id.clone(),
    );

    GatewayAcpDispatchTargetReadModel {
        original_session_id: target.original_session_id.clone(),
        route_session_id: target.route_session_id.clone(),
        prefixed_agent_id: target.prefixed_agent_id.clone(),
        channel_id: route.channel_id,
        account_id: route.account_id,
        conversation_id: route.conversation_id,
        participant_id: route.participant_id,
        thread_id: route.thread_id,
        channel_path: target.channel_path.clone(),
    }
}

struct GatewayAcpRouteProjection {
    channel_id: Option<String>,
    account_id: Option<String>,
    conversation_id: Option<String>,
    participant_id: Option<String>,
    thread_id: Option<String>,
}

fn build_acp_route_projection(
    channel_id: Option<String>,
    account_id: Option<String>,
    conversation_id: Option<String>,
    participant_id: Option<String>,
    thread_id: Option<String>,
) -> GatewayAcpRouteProjection {
    GatewayAcpRouteProjection {
        channel_id,
        account_id,
        conversation_id,
        participant_id,
        thread_id,
    }
}

fn build_acp_dispatch_decision_read_model(
    session_id: &str,
    decision: &app::acp::AcpConversationDispatchDecision,
) -> GatewayAcpDispatchDecisionReadModel {
    let session = session_id.to_owned();
    let route_via_acp = decision.route_via_acp;
    let reason = decision.reason.as_str();
    let automatic_routing_origin = decision
        .automatic_routing_origin
        .map(app::acp::AcpRoutingOrigin::as_str);
    let provenance = build_acp_dispatch_prediction_provenance_read_model(decision);
    let target = build_acp_dispatch_target_read_model(&decision.target);
    let decision = GatewayAcpDispatchDecisionDetailsReadModel {
        route_via_acp,
        reason,
        automatic_routing_origin,
        provenance,
        target,
    };

    GatewayAcpDispatchDecisionReadModel { session, decision }
}
