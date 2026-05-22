use serde::{Deserialize, Serialize};

use crate::app;
use crate::gateway::state::GatewayOwnerStatus;

use super::pairing::{
    GatewayOperatorNodesSummaryReadModel, GatewayOperatorPairingSummaryReadModel,
    build_gateway_node_inventory_from_registry_read_model, build_gateway_pairing_summary_read_model,
    build_operator_nodes_summary_read_model,
};
use super::{
    GatewayChannelInventoryReadModel, GatewayRuntimeSnapshotReadModel, GatewayToolAccessReadModel,
    GatewayToolCallingReadModel, build_operator_channel_surface_read_models, core,
    summarize_operator_runtime_enabled_channels,
    summarize_operator_runtime_provider, summarize_operator_runtime_tools,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorControlSurfaceReadModel {
    pub base_url: Option<String>,
    pub loopback_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorRuntimeIncidentReadModel {
    pub account_id: Option<String>,
    pub account_label: Option<String>,
    pub kind: String,
    pub at_ms: u64,
    pub detail: Option<String>,
    pub owner_pids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorChannelSurfaceReadModel {
    pub channel_id: String,
    pub label: String,
    pub implementation_status: String,
    pub runtime_kind: String,
    pub operational_model: String,
    pub service_contract_model: String,
    pub configured_account_count: usize,
    pub enabled_account_count: usize,
    pub misconfigured_account_count: usize,
    pub ready_send_account_count: usize,
    pub ready_serve_account_count: usize,
    pub conversation_gated_account_count: usize,
    pub sender_gated_account_count: usize,
    pub mention_gated_account_count: usize,
    pub default_configured_account_id: Option<String>,
    pub plugin_bridge_account_summary: Option<String>,
    pub runtime_attention_account_count: usize,
    pub runtime_attention_reasons: Vec<String>,
    pub runtime_attention_remediations: Vec<String>,
    pub retrying_runtime_account_count: usize,
    pub stale_runtime_account_count: usize,
    pub duplicate_runtime_account_count: usize,
    pub preferred_runtime_owner_pids: Vec<u32>,
    pub duplicate_runtime_cleanup_owner_pids: Vec<u32>,
    pub last_duplicate_runtime_auto_reclaim_at: Option<u64>,
    pub last_duplicate_runtime_auto_cleanup_owner_pids: Vec<u32>,
    pub recent_runtime_incidents: Vec<GatewayOperatorRuntimeIncidentReadModel>,
    pub service_enabled: bool,
    pub service_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorChannelsSummaryReadModel {
    pub catalog_channel_count: usize,
    pub configured_channel_count: usize,
    pub configured_account_count: usize,
    pub enabled_account_count: usize,
    pub misconfigured_account_count: usize,
    pub runtime_backed_channel_count: usize,
    pub config_backed_channel_count: usize,
    pub plugin_backed_channel_count: usize,
    pub catalog_only_channel_count: usize,
    pub gateway_supervised_channel_count: usize,
    pub standalone_runtime_channel_count: usize,
    pub managed_bridge_capable_service_channel_count: usize,
    pub native_service_channel_count: usize,
    pub standalone_native_service_channel_count: usize,
    pub external_plugin_bridge_channel_count: usize,
    pub direct_send_only_channel_count: usize,
    pub catalog_only_service_contract_channel_count: usize,
    pub enabled_runtime_backed_channel_count: usize,
    pub enabled_plugin_backed_channel_count: usize,
    pub enabled_outbound_only_channel_count: usize,
    pub enabled_service_channel_count: usize,
    pub ready_service_channel_count: usize,
    pub runtime_attention_surface_count: usize,
    pub retrying_runtime_surface_count: usize,
    pub stale_runtime_surface_count: usize,
    pub duplicate_runtime_surface_count: usize,
    pub runtime_attention_surface_ids: Vec<String>,
    pub retrying_runtime_surface_ids: Vec<String>,
    pub stale_runtime_surface_ids: Vec<String>,
    pub duplicate_runtime_surface_ids: Vec<String>,
    pub surfaces: Vec<GatewayOperatorChannelSurfaceReadModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorRuntimeSummaryReadModel {
    pub enabled_channel_ids: Vec<String>,
    pub enabled_runtime_backed_channel_ids: Vec<String>,
    pub enabled_service_channel_ids: Vec<String>,
    pub enabled_plugin_backed_channel_ids: Vec<String>,
    pub enabled_outbound_only_channel_ids: Vec<String>,
    pub visible_tool_count: usize,
    pub visible_direct_tool_names: Vec<String>,
    pub hidden_tool_surface_ids: Vec<String>,
    pub capability_snapshot_sha256: String,
    pub active_provider_profile_id: Option<String>,
    pub active_provider_label: Option<String>,
    pub compaction_hygiene: crate::RuntimeSnapshotCompactionHygieneState,
    pub tool_calling: GatewayToolCallingReadModel,
    pub access: GatewayToolAccessReadModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayOperatorSummaryReadModel {
    pub owner: GatewayOwnerStatus,
    pub control_surface: GatewayOperatorControlSurfaceReadModel,
    pub channels: GatewayOperatorChannelsSummaryReadModel,
    pub runtime: GatewayOperatorRuntimeSummaryReadModel,
    pub pairing: GatewayOperatorPairingSummaryReadModel,
    pub nodes: GatewayOperatorNodesSummaryReadModel,
}

pub fn build_operator_summary_read_model(
    owner_status: &GatewayOwnerStatus,
    channel_inventory: &GatewayChannelInventoryReadModel,
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
    pairing: GatewayOperatorPairingSummaryReadModel,
    nodes: GatewayOperatorNodesSummaryReadModel,
) -> GatewayOperatorSummaryReadModel {
    let owner = owner_status.clone();
    let control_surface = build_operator_control_surface_read_model(owner_status);
    let channels = build_operator_channels_summary_read_model(channel_inventory, runtime_snapshot);
    let runtime = build_operator_runtime_summary_read_model(runtime_snapshot);

    GatewayOperatorSummaryReadModel {
        owner,
        control_surface,
        channels,
        runtime,
        pairing,
        nodes,
    }
}

pub fn build_gateway_operator_summary_from_registry_read_model(
    owner_status: &GatewayOwnerStatus,
    channel_inventory: &GatewayChannelInventoryReadModel,
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
    config_path: &str,
    pairing_registry: Option<&app::control_plane::ControlPlanePairingRegistry>,
) -> GatewayOperatorSummaryReadModel {
    let pairing = build_gateway_pairing_summary_read_model(pairing_registry);
    let node_inventory =
        build_gateway_node_inventory_from_registry_read_model(config_path, channel_inventory, pairing_registry);
    let nodes = build_operator_nodes_summary_read_model(&node_inventory);
    build_operator_summary_read_model(owner_status, channel_inventory, runtime_snapshot, pairing, nodes)
}

pub(crate) fn build_operator_control_surface_read_model(
    owner_status: &GatewayOwnerStatus,
) -> GatewayOperatorControlSurfaceReadModel {
    let base_url = core::gateway_owner_base_url(owner_status);
    let loopback_only = core::gateway_owner_control_is_loopback(owner_status);

    GatewayOperatorControlSurfaceReadModel {
        base_url,
        loopback_only,
    }
}

pub(crate) fn build_operator_channels_summary_read_model(
    channel_inventory: &GatewayChannelInventoryReadModel,
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
) -> GatewayOperatorChannelsSummaryReadModel {
    let catalog_counts = summarize_operator_channel_catalog(channel_inventory);
    let enabled_counts = summarize_operator_enabled_channels(runtime_snapshot);
    let catalog_channel_count = channel_inventory.channel_catalog.len();
    let configured_channel_count = channel_inventory
        .channel_surfaces
        .iter()
        .filter(|surface| !surface.surface.configured_accounts.is_empty())
        .count();
    let configured_account_count = channel_inventory.channels.len();
    let enabled_account_count = channel_inventory
        .channels
        .iter()
        .filter(|account| account.enabled)
        .count();
    let misconfigured_account_count = channel_inventory
        .channels
        .iter()
        .filter(|account| core::channel_account_is_misconfigured(account))
        .count();
    let runtime_backed_channel_count = catalog_counts.runtime_backed_channel_count;
    let config_backed_channel_count = catalog_counts.config_backed_channel_count;
    let plugin_backed_channel_count = catalog_counts.plugin_backed_channel_count;
    let catalog_only_channel_count = catalog_counts.catalog_only_channel_count;
    let gateway_supervised_channel_count = catalog_counts.gateway_supervised_channel_count;
    let standalone_runtime_channel_count = catalog_counts.standalone_runtime_channel_count;
    let managed_bridge_capable_service_channel_count =
        catalog_counts.managed_bridge_capable_service_channel_count;
    let native_service_channel_count = catalog_counts.native_service_channel_count;
    let standalone_native_service_channel_count =
        catalog_counts.standalone_native_service_channel_count;
    let external_plugin_bridge_channel_count = catalog_counts.external_plugin_bridge_channel_count;
    let direct_send_only_channel_count = catalog_counts.direct_send_only_channel_count;
    let catalog_only_service_contract_channel_count =
        catalog_counts.catalog_only_service_contract_channel_count;
    let enabled_service_channel_ids = &enabled_counts.enabled_service_channel_ids;
    let enabled_runtime_backed_channel_count = enabled_counts.enabled_runtime_backed_channel_count;
    let enabled_plugin_backed_channel_count = enabled_counts.enabled_plugin_backed_channel_count;
    let enabled_outbound_only_channel_count = enabled_counts.enabled_outbound_only_channel_count;
    let enabled_service_channel_count = enabled_counts.enabled_service_channel_count;
    let surfaces = build_operator_channel_surface_read_models(
        &channel_inventory.channel_surfaces,
        &channel_inventory.channel_access_policies,
        enabled_service_channel_ids,
    );
    let runtime_rollups = summarize_operator_channel_runtime_rollups(&surfaces);

    GatewayOperatorChannelsSummaryReadModel {
        catalog_channel_count,
        configured_channel_count,
        configured_account_count,
        enabled_account_count,
        misconfigured_account_count,
        runtime_backed_channel_count,
        config_backed_channel_count,
        plugin_backed_channel_count,
        catalog_only_channel_count,
        gateway_supervised_channel_count,
        standalone_runtime_channel_count,
        managed_bridge_capable_service_channel_count,
        native_service_channel_count,
        standalone_native_service_channel_count,
        external_plugin_bridge_channel_count,
        direct_send_only_channel_count,
        catalog_only_service_contract_channel_count,
        enabled_runtime_backed_channel_count,
        enabled_plugin_backed_channel_count,
        enabled_outbound_only_channel_count,
        enabled_service_channel_count,
        ready_service_channel_count: runtime_rollups.ready_service_channel_count,
        runtime_attention_surface_count: runtime_rollups.runtime_attention_surface_count,
        retrying_runtime_surface_count: runtime_rollups.retrying_runtime_surface_count,
        stale_runtime_surface_count: runtime_rollups.stale_runtime_surface_count,
        duplicate_runtime_surface_count: runtime_rollups.duplicate_runtime_surface_count,
        runtime_attention_surface_ids: runtime_rollups.runtime_attention_surface_ids,
        retrying_runtime_surface_ids: runtime_rollups.retrying_runtime_surface_ids,
        stale_runtime_surface_ids: runtime_rollups.stale_runtime_surface_ids,
        duplicate_runtime_surface_ids: runtime_rollups.duplicate_runtime_surface_ids,
        surfaces,
    }
}

pub(crate) fn build_operator_runtime_summary_read_model(
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
) -> GatewayOperatorRuntimeSummaryReadModel {
    let enabled_channel_ids = runtime_snapshot.channels.enabled_channel_ids.clone();
    let enabled_channel_rollups = summarize_operator_runtime_enabled_channels(runtime_snapshot);
    let tool_rollups = summarize_operator_runtime_tools(runtime_snapshot);
    let provider_rollups = summarize_operator_runtime_provider(runtime_snapshot);

    GatewayOperatorRuntimeSummaryReadModel {
        enabled_channel_ids,
        enabled_runtime_backed_channel_ids:
            enabled_channel_rollups.enabled_runtime_backed_channel_ids,
        enabled_service_channel_ids: enabled_channel_rollups.enabled_service_channel_ids,
        enabled_plugin_backed_channel_ids: enabled_channel_rollups.enabled_plugin_backed_channel_ids,
        enabled_outbound_only_channel_ids:
            enabled_channel_rollups.enabled_outbound_only_channel_ids,
        visible_tool_count: tool_rollups.visible_tool_count,
        visible_direct_tool_names: tool_rollups.visible_direct_tool_names,
        hidden_tool_surface_ids: tool_rollups.hidden_tool_surface_ids,
        capability_snapshot_sha256: tool_rollups.capability_snapshot_sha256,
        active_provider_profile_id: provider_rollups.active_provider_profile_id,
        active_provider_label: provider_rollups.active_provider_label,
        compaction_hygiene: provider_rollups.compaction_hygiene,
        tool_calling: tool_rollups.tool_calling,
        access: tool_rollups.access,
    }
}

struct GatewayOperatorChannelCatalogCounts {
    runtime_backed_channel_count: usize,
    config_backed_channel_count: usize,
    plugin_backed_channel_count: usize,
    catalog_only_channel_count: usize,
    gateway_supervised_channel_count: usize,
    standalone_runtime_channel_count: usize,
    managed_bridge_capable_service_channel_count: usize,
    native_service_channel_count: usize,
    standalone_native_service_channel_count: usize,
    external_plugin_bridge_channel_count: usize,
    direct_send_only_channel_count: usize,
    catalog_only_service_contract_channel_count: usize,
}

pub(crate) fn summarize_operator_channel_catalog(
    channel_inventory: &GatewayChannelInventoryReadModel,
) -> GatewayOperatorChannelCatalogCounts {
    GatewayOperatorChannelCatalogCounts {
        runtime_backed_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.runtime_kind == "runtime_backed")
            .count(),
        config_backed_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.runtime_kind == "outbound_only")
            .count(),
        plugin_backed_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| {
                channel.catalog.implementation_status
                    == app::channel::ChannelCatalogImplementationStatus::PluginBacked
            })
            .count(),
        catalog_only_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.runtime_kind == "catalog_only")
            .count(),
        gateway_supervised_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.operational_model == "gateway_supervised")
            .count(),
        standalone_runtime_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.operational_model == "standalone_runtime")
            .count(),
        managed_bridge_capable_service_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.service_contract_model == "managed_bridge_capable_service")
            .count(),
        native_service_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.service_contract_model == "native_service_channel")
            .count(),
        standalone_native_service_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.service_contract_model == "standalone_native_service")
            .count(),
        external_plugin_bridge_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.service_contract_model == "external_plugin_bridge")
            .count(),
        direct_send_only_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.service_contract_model == "direct_send_only")
            .count(),
        catalog_only_service_contract_channel_count: channel_inventory
            .channel_catalog
            .iter()
            .filter(|channel| channel.service_contract_model == "catalog_only")
            .count(),
    }
}

pub(crate) struct GatewayOperatorEnabledChannelCounts<'a> {
    pub enabled_service_channel_ids: &'a Vec<String>,
    pub enabled_runtime_backed_channel_count: usize,
    pub enabled_plugin_backed_channel_count: usize,
    pub enabled_outbound_only_channel_count: usize,
    pub enabled_service_channel_count: usize,
}

pub(crate) fn summarize_operator_enabled_channels(
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
) -> GatewayOperatorEnabledChannelCounts<'_> {
    let enabled_runtime_backed_channel_ids =
        &runtime_snapshot.channels.enabled_runtime_backed_channel_ids;
    let enabled_plugin_backed_channel_ids =
        &runtime_snapshot.channels.enabled_plugin_backed_channel_ids;
    let enabled_outbound_only_channel_ids =
        &runtime_snapshot.channels.enabled_outbound_only_channel_ids;
    let enabled_service_channel_ids = &runtime_snapshot.channels.enabled_service_channel_ids;

    GatewayOperatorEnabledChannelCounts {
        enabled_service_channel_ids,
        enabled_runtime_backed_channel_count: enabled_runtime_backed_channel_ids.len(),
        enabled_plugin_backed_channel_count: enabled_plugin_backed_channel_ids.len(),
        enabled_outbound_only_channel_count: enabled_outbound_only_channel_ids.len(),
        enabled_service_channel_count: enabled_service_channel_ids.len(),
    }
}

pub(crate) struct GatewayOperatorChannelRuntimeRollups {
    pub ready_service_channel_count: usize,
    pub runtime_attention_surface_count: usize,
    pub retrying_runtime_surface_count: usize,
    pub stale_runtime_surface_count: usize,
    pub duplicate_runtime_surface_count: usize,
    pub runtime_attention_surface_ids: Vec<String>,
    pub retrying_runtime_surface_ids: Vec<String>,
    pub stale_runtime_surface_ids: Vec<String>,
    pub duplicate_runtime_surface_ids: Vec<String>,
}

pub(crate) fn summarize_operator_channel_runtime_rollups(
    surfaces: &[GatewayOperatorChannelSurfaceReadModel],
) -> GatewayOperatorChannelRuntimeRollups {
    GatewayOperatorChannelRuntimeRollups {
        ready_service_channel_count: surfaces
            .iter()
            .filter(|surface| surface.service_ready)
            .count(),
        runtime_attention_surface_count: surfaces
            .iter()
            .filter(|surface| surface.runtime_attention_account_count > 0)
            .count(),
        retrying_runtime_surface_count: surfaces
            .iter()
            .filter(|surface| surface.retrying_runtime_account_count > 0)
            .count(),
        stale_runtime_surface_count: surfaces
            .iter()
            .filter(|surface| surface.stale_runtime_account_count > 0)
            .count(),
        duplicate_runtime_surface_count: surfaces
            .iter()
            .filter(|surface| surface.duplicate_runtime_account_count > 0)
            .count(),
        runtime_attention_surface_ids: surfaces
            .iter()
            .filter(|surface| surface.runtime_attention_account_count > 0)
            .map(|surface| surface.channel_id.clone())
            .collect(),
        retrying_runtime_surface_ids: surfaces
            .iter()
            .filter(|surface| surface.retrying_runtime_account_count > 0)
            .map(|surface| surface.channel_id.clone())
            .collect(),
        stale_runtime_surface_ids: surfaces
            .iter()
            .filter(|surface| surface.stale_runtime_account_count > 0)
            .map(|surface| surface.channel_id.clone())
            .collect(),
        duplicate_runtime_surface_ids: surfaces
            .iter()
            .filter(|surface| surface.duplicate_runtime_account_count > 0)
            .map(|surface| surface.channel_id.clone())
            .collect(),
    }
}
