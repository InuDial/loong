use loong_protocol::{
    CONTROL_PLANE_PROTOCOL_VERSION, ControlPlaneChallengeResponse, ControlPlanePrincipal,
    ControlPlaneRole, ControlPlaneScope,
};
use serde::{Deserialize, Serialize};

use crate::app;

use super::GatewayChannelInventoryReadModel;
use crate::gateway::event_bus::{GatewayEventRecord, GatewayEventReplayWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorPairingSummaryReadModel {
    pub pending_request_count: usize,
    pub approved_device_count: usize,
    pub last_activity_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayNodeInventorySummaryReadModel {
    pub paired_device_count: usize,
    pub managed_bridge_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorNodesSummaryReadModel {
    pub paired_device_count: usize,
    pub managed_bridge_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPairedDeviceNodeReadModel {
    pub node_id: String,
    pub node_kind: String,
    pub trust_state: String,
    pub role: String,
    pub public_key: String,
    pub approved_scopes: Vec<String>,
    pub issued_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayManagedBridgeNodeReadModel {
    pub node_id: String,
    pub node_kind: String,
    pub trust_state: String,
    pub channel_id: String,
    pub implementation_status: String,
    pub configured_account_count: usize,
    pub enabled_account_count: usize,
    pub configured_plugin_id: Option<String>,
    pub selected_plugin_id: Option<String>,
    pub discovery_status: Option<String>,
    pub selection_status: Option<String>,
    pub compatible_plugins: usize,
    pub incomplete_plugins: usize,
    pub incompatible_plugins: usize,
    pub account_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayNodeInventoryReadModel {
    pub config: String,
    pub summary: GatewayNodeInventorySummaryReadModel,
    pub paired_devices: Vec<GatewayPairedDeviceNodeReadModel>,
    pub managed_bridges: Vec<GatewayManagedBridgeNodeReadModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPairingStartReadModel {
    pub protocol: u32,
    pub challenge: ControlPlaneChallengeResponse,
    pub connect_path: String,
    pub pairing_requests_path: String,
    pub pairing_resolve_path: String,
    pub recommended_role: ControlPlaneRole,
    pub recommended_scopes: Vec<ControlPlaneScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPairingCompleteReadModel {
    pub status: String,
    pub device_id: String,
    pub client_id: String,
    pub role: ControlPlaneRole,
    pub requested_scopes: Vec<ControlPlaneScope>,
    pub lease: GatewayPairingSessionLeaseReadModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPairingSessionLeaseReadModel {
    pub connection_token: String,
    pub connection_token_expires_at_ms: u64,
    pub principal: ControlPlanePrincipal,
    pub last_acknowledged_seq: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPairingReplayWindowReadModel {
    pub oldest_retained_seq: Option<u64>,
    pub latest_seq: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPairingSessionReadModel {
    pub status: String,
    pub connection_token_expires_at_ms: u64,
    pub principal: ControlPlanePrincipal,
    pub last_acknowledged_seq: Option<u64>,
    pub resume_status: String,
    pub resume_from_after_seq: u64,
    pub earliest_resumable_after_seq: u64,
    pub replay_window: GatewayPairingReplayWindowReadModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPairingEventsReadModel {
    pub after_seq: u64,
    pub effective_after_seq: u64,
    pub returned_count: usize,
    pub last_acknowledged_seq: Option<u64>,
    pub resume_status: String,
    pub next_after_seq: u64,
    pub earliest_resumable_after_seq: u64,
    pub replay_window: GatewayPairingReplayWindowReadModel,
    pub events: Vec<GatewayEventRecord>,
}

pub fn build_node_inventory_read_model(
    config_path: &str,
    channel_inventory: &GatewayChannelInventoryReadModel,
    paired_devices: &[app::control_plane::ControlPlaneApprovedDeviceSummary],
) -> GatewayNodeInventoryReadModel {
    let config = config_path.to_owned();
    let paired_devices = build_paired_device_nodes_read_model(paired_devices);
    let managed_bridges = build_managed_bridge_nodes_read_model(channel_inventory);
    let summary = GatewayNodeInventorySummaryReadModel {
        paired_device_count: paired_devices.len(),
        managed_bridge_count: managed_bridges.len(),
        total_count: paired_devices.len() + managed_bridges.len(),
    };

    GatewayNodeInventoryReadModel {
        config,
        summary,
        paired_devices,
        managed_bridges,
    }
}

pub fn build_operator_nodes_summary_read_model(
    inventory: &GatewayNodeInventoryReadModel,
) -> GatewayOperatorNodesSummaryReadModel {
    GatewayOperatorNodesSummaryReadModel {
        paired_device_count: inventory.summary.paired_device_count,
        managed_bridge_count: inventory.summary.managed_bridge_count,
        total_count: inventory.summary.total_count,
    }
}

pub fn build_gateway_pairing_summary_read_model(
    pairing_registry: Option<&app::control_plane::ControlPlanePairingRegistry>,
) -> GatewayOperatorPairingSummaryReadModel {
    match pairing_registry {
        Some(pairing_registry) => GatewayOperatorPairingSummaryReadModel {
            pending_request_count: pairing_registry.pending_request_count(),
            approved_device_count: pairing_registry.approved_device_count(),
            last_activity_ms: pairing_registry.last_activity_ms(),
        },
        None => GatewayOperatorPairingSummaryReadModel {
            pending_request_count: 0,
            approved_device_count: 0,
            last_activity_ms: None,
        },
    }
}

pub fn build_gateway_node_inventory_from_registry_read_model(
    config_path: &str,
    channel_inventory: &GatewayChannelInventoryReadModel,
    pairing_registry: Option<&app::control_plane::ControlPlanePairingRegistry>,
) -> GatewayNodeInventoryReadModel {
    let paired_devices = pairing_registry
        .map(|registry| registry.list_approved_devices(256))
        .unwrap_or_default();
    build_node_inventory_read_model(config_path, channel_inventory, paired_devices.as_slice())
}

pub fn build_gateway_pairing_start_read_model(
    challenge: ControlPlaneChallengeResponse,
) -> GatewayPairingStartReadModel {
    GatewayPairingStartReadModel {
        protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        challenge,
        connect_path: "/v1/pairing/complete".to_owned(),
        pairing_requests_path: "/v1/pairing/requests".to_owned(),
        pairing_resolve_path: "/v1/pairing/resolve".to_owned(),
        recommended_role: ControlPlaneRole::Operator,
        recommended_scopes: vec![
            ControlPlaneScope::OperatorRead,
            ControlPlaneScope::OperatorPairing,
        ],
    }
}

pub fn build_gateway_pairing_complete_read_model(
    device_id: &str,
    client_id: &str,
    role: ControlPlaneRole,
    requested_scopes: Vec<ControlPlaneScope>,
    lease: GatewayPairingSessionLeaseReadModel,
) -> GatewayPairingCompleteReadModel {
    GatewayPairingCompleteReadModel {
        status: "authorized".to_owned(),
        device_id: device_id.to_owned(),
        client_id: client_id.to_owned(),
        role,
        requested_scopes,
        lease,
    }
}

pub fn build_gateway_pairing_session_read_model(
    lease: GatewayPairingSessionLeaseReadModel,
    replay_window: GatewayEventReplayWindow,
) -> GatewayPairingSessionReadModel {
    let earliest_resumable_after_seq = replay_window
        .oldest_retained_seq
        .map(|seq| seq.saturating_sub(1))
        .unwrap_or(0);
    let (resume_status, resume_from_after_seq) =
        gateway_pairing_resume_contract(lease.last_acknowledged_seq, replay_window);

    GatewayPairingSessionReadModel {
        status: "active".to_owned(),
        connection_token_expires_at_ms: lease.connection_token_expires_at_ms,
        principal: lease.principal,
        last_acknowledged_seq: lease.last_acknowledged_seq,
        resume_status: resume_status.to_owned(),
        resume_from_after_seq,
        earliest_resumable_after_seq,
        replay_window: build_gateway_pairing_replay_window_read_model(replay_window),
    }
}

pub fn build_gateway_pairing_events_read_model(
    after_seq: u64,
    last_acknowledged_seq: Option<u64>,
    replay_window: GatewayEventReplayWindow,
    events: Vec<GatewayEventRecord>,
) -> GatewayPairingEventsReadModel {
    let returned_count = events.len();
    let next_after_seq = events.last().map(|event| event.seq).unwrap_or(after_seq);
    let earliest_resumable_after_seq = replay_window
        .oldest_retained_seq
        .map(|seq| seq.saturating_sub(1))
        .unwrap_or(0);
    let resume_status = if after_seq == 0 { "fresh" } else { "resumed" };

    GatewayPairingEventsReadModel {
        after_seq,
        effective_after_seq: after_seq,
        returned_count,
        last_acknowledged_seq,
        resume_status: resume_status.to_owned(),
        next_after_seq,
        earliest_resumable_after_seq,
        replay_window: build_gateway_pairing_replay_window_read_model(replay_window),
        events,
    }
}

pub fn build_gateway_pairing_replay_window_read_model(
    replay_window: GatewayEventReplayWindow,
) -> GatewayPairingReplayWindowReadModel {
    GatewayPairingReplayWindowReadModel {
        oldest_retained_seq: replay_window.oldest_retained_seq,
        latest_seq: replay_window.latest_seq,
    }
}

fn gateway_pairing_resume_contract(
    last_acknowledged_seq: Option<u64>,
    replay_window: GatewayEventReplayWindow,
) -> (&'static str, u64) {
    let earliest_resumable_after_seq = replay_window
        .oldest_retained_seq
        .map(|seq| seq.saturating_sub(1))
        .unwrap_or(0);
    let Some(last_acknowledged_seq) = last_acknowledged_seq else {
        return ("fresh", 0);
    };

    if replay_window.oldest_retained_seq.is_some()
        && last_acknowledged_seq < earliest_resumable_after_seq
    {
        return ("stale", earliest_resumable_after_seq);
    }

    ("resumed", last_acknowledged_seq)
}

fn build_paired_device_nodes_read_model(
    paired_devices: &[app::control_plane::ControlPlaneApprovedDeviceSummary],
) -> Vec<GatewayPairedDeviceNodeReadModel> {
    paired_devices
        .iter()
        .map(|device| GatewayPairedDeviceNodeReadModel {
            node_id: device.device_id.clone(),
            node_kind: paired_device_node_kind(device.role.as_str()).to_owned(),
            trust_state: "paired".to_owned(),
            role: device.role.clone(),
            public_key: device.public_key.clone(),
            approved_scopes: device.approved_scopes.iter().cloned().collect(),
            issued_at_ms: device.issued_at_ms,
        })
        .collect()
}

fn paired_device_node_kind(role: &str) -> &'static str {
    match role {
        "operator" => "operator_ui",
        _ => "node_client",
    }
}

fn build_managed_bridge_nodes_read_model(
    channel_inventory: &GatewayChannelInventoryReadModel,
) -> Vec<GatewayManagedBridgeNodeReadModel> {
    let mut nodes = channel_inventory
        .channel_surfaces
        .iter()
        .filter_map(build_managed_bridge_node_read_model)
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.channel_id.cmp(&right.channel_id));
    nodes
}

fn build_managed_bridge_node_read_model(
    surface: &super::GatewayChannelSurfaceReadModel,
) -> Option<GatewayManagedBridgeNodeReadModel> {
    let discovery = surface.surface.plugin_bridge_discovery.as_ref()?;
    let enabled_account_count = count_enabled_channel_accounts(surface);
    if !channel_surface_has_operator_relevant_bridge_surface(discovery, enabled_account_count) {
        return None;
    }

    Some(GatewayManagedBridgeNodeReadModel {
        node_id: format!("managed_bridge:{}", surface.surface.catalog.id),
        node_kind: "managed_bridge".to_owned(),
        trust_state: managed_bridge_trust_state(discovery).to_owned(),
        channel_id: surface.surface.catalog.id.to_owned(),
        implementation_status: surface.surface.catalog.implementation_status.as_str().to_owned(),
        configured_account_count: surface.surface.configured_accounts.len(),
        enabled_account_count,
        configured_plugin_id: discovery.configured_plugin_id.clone(),
        selected_plugin_id: discovery.selected_plugin_id.clone(),
        discovery_status: Some(discovery.status.as_str().to_owned()),
        selection_status: discovery
            .selection_status
            .map(|status| status.as_str().to_owned()),
        compatible_plugins: discovery.compatible_plugins,
        incomplete_plugins: discovery.incomplete_plugins,
        incompatible_plugins: discovery.incompatible_plugins,
        account_summary: surface.plugin_bridge_account_summary.clone(),
    })
}

fn count_enabled_channel_accounts(surface: &super::GatewayChannelSurfaceReadModel) -> usize {
    surface
        .surface
        .configured_accounts
        .iter()
        .filter(|snapshot| snapshot.enabled)
        .count()
}

fn channel_surface_has_operator_relevant_bridge_surface(
    discovery: &app::channel::ChannelPluginBridgeDiscovery,
    enabled_account_count: usize,
) -> bool {
    enabled_account_count > 0
        || discovery.selected_plugin_id.is_some()
        || discovery.configured_plugin_id.is_some()
        || discovery.compatible_plugins > 0
}

fn managed_bridge_trust_state(
    discovery: &app::channel::ChannelPluginBridgeDiscovery,
) -> &'static str {
    use app::channel::ChannelPluginBridgeDiscoveryStatus as DiscoveryStatus;

    match discovery.status {
        DiscoveryStatus::NotConfigured => "not_configured",
        DiscoveryStatus::ScanFailed => "scan_failed",
        DiscoveryStatus::NoMatches => "unresolved",
        DiscoveryStatus::MatchesFound => {
            if discovery
                .selection_status
                .is_some_and(|status| status.selects_ready_plugin())
            {
                "ready"
            } else {
                "review_required"
            }
        }
    }
}
