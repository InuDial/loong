use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app;
use crate::gateway::state::GatewayOwnerStatus;

use super::pairing::{
    GatewayOperatorNodesSummaryReadModel, GatewayOperatorPairingSummaryReadModel,
    build_gateway_node_inventory_from_registry_read_model, build_gateway_pairing_summary_read_model,
    build_operator_nodes_summary_read_model,
};
use super::{
    GatewayChannelInventoryReadModel, GatewayRuntimeSnapshotReadModel, GatewayToolAccessReadModel,
    GatewayToolCallingReadModel, core,
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

struct GatewayOperatorRuntimeEnabledChannels {
    enabled_runtime_backed_channel_ids: Vec<String>,
    enabled_service_channel_ids: Vec<String>,
    enabled_plugin_backed_channel_ids: Vec<String>,
    enabled_outbound_only_channel_ids: Vec<String>,
}

fn summarize_operator_runtime_enabled_channels(
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
) -> GatewayOperatorRuntimeEnabledChannels {
    GatewayOperatorRuntimeEnabledChannels {
        enabled_runtime_backed_channel_ids: runtime_snapshot
            .channels
            .enabled_runtime_backed_channel_ids
            .clone(),
        enabled_service_channel_ids: runtime_snapshot.channels.enabled_service_channel_ids.clone(),
        enabled_plugin_backed_channel_ids: runtime_snapshot
            .channels
            .enabled_plugin_backed_channel_ids
            .clone(),
        enabled_outbound_only_channel_ids: runtime_snapshot
            .channels
            .enabled_outbound_only_channel_ids
            .clone(),
    }
}

struct GatewayOperatorRuntimeToolRollups {
    visible_tool_count: usize,
    visible_direct_tool_names: Vec<String>,
    hidden_tool_surface_ids: Vec<String>,
    capability_snapshot_sha256: String,
    tool_calling: GatewayToolCallingReadModel,
    access: GatewayToolAccessReadModel,
}

fn summarize_operator_runtime_tools(
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
) -> GatewayOperatorRuntimeToolRollups {
    GatewayOperatorRuntimeToolRollups {
        visible_tool_count: runtime_snapshot.tools.visible_tool_count,
        visible_direct_tool_names: runtime_snapshot.tools.visible_direct_tool_names.clone(),
        hidden_tool_surface_ids: runtime_snapshot
            .tools
            .hidden_tool_surfaces
            .iter()
            .map(|surface| surface.surface_id.clone())
            .collect(),
        capability_snapshot_sha256: runtime_snapshot.tools.capability_snapshot_sha256.clone(),
        tool_calling: runtime_snapshot.tools.tool_calling.clone(),
        access: runtime_snapshot.tools.access.clone(),
    }
}

struct GatewayOperatorRuntimeProviderRollups {
    active_provider_profile_id: Option<String>,
    active_provider_label: Option<String>,
    compaction_hygiene: crate::RuntimeSnapshotCompactionHygieneState,
}

fn summarize_operator_runtime_provider(
    runtime_snapshot: &GatewayRuntimeSnapshotReadModel,
) -> GatewayOperatorRuntimeProviderRollups {
    let active_provider_profile_id =
        json_string_field(&runtime_snapshot.provider, "active_profile_id");
    let active_provider_label = json_string_field(&runtime_snapshot.provider, "active_label");
    let compaction_hygiene = runtime_snapshot.context_engine.get("compaction_hygiene");
    let compaction_hygiene =
        crate::RuntimeSnapshotCompactionHygieneState::decode_or_unknown(compaction_hygiene);

    GatewayOperatorRuntimeProviderRollups {
        active_provider_profile_id,
        active_provider_label,
        compaction_hygiene,
    }
}

fn channel_account_operation_is_ready(
    account: &app::channel::ChannelStatusSnapshot,
    operation_id: &str,
) -> bool {
    let operation = account.operation(operation_id);
    let Some(operation) = operation else {
        return false;
    };

    operation.health == app::channel::ChannelOperationHealth::Ready
}

fn channel_account_serve_runtime(
    account: &app::channel::ChannelStatusSnapshot,
) -> Option<&app::channel::ChannelOperationRuntime> {
    account
        .operation(app::channel::CHANNEL_OPERATION_SERVE_ID)
        .and_then(|operation| operation.runtime.as_ref())
}

fn channel_account_has_runtime_attention(account: &app::channel::ChannelStatusSnapshot) -> bool {
    channel_account_has_retrying_runtime(account)
        || channel_account_has_stale_runtime(account)
        || channel_account_has_duplicate_runtime(account)
}

fn collect_channel_surface_runtime_attention_reasons(
    surface: &app::channel::ChannelSurface,
) -> Vec<String> {
    let runtime_flags = summarize_channel_surface_runtime_flags(surface);
    runtime_attention_reasons_from_flags(&runtime_flags)
}

fn runtime_attention_reason_remediation(reason: &str) -> &'static str {
    match reason {
        "retrying" => "inspect_bridge_connectivity",
        "stale" => "restart_stale_runtime",
        "duplicate_runtime_instances" => "stop_duplicate_runtime_instances",
        _ => "inspect_runtime_attention",
    }
}

fn channel_account_has_retrying_runtime(account: &app::channel::ChannelStatusSnapshot) -> bool {
    channel_account_serve_runtime(account)
        .map(|runtime| runtime.running && runtime.consecutive_failures > 0)
        .unwrap_or(false)
}

fn channel_account_has_stale_runtime(account: &app::channel::ChannelStatusSnapshot) -> bool {
    channel_account_serve_runtime(account)
        .map(|runtime| runtime.stale)
        .unwrap_or(false)
}

fn channel_account_has_duplicate_runtime(account: &app::channel::ChannelStatusSnapshot) -> bool {
    channel_account_serve_runtime(account)
        .map(|runtime| runtime.running_instances > 1)
        .unwrap_or(false)
}

struct ChannelSurfaceRuntimeFlags {
    has_retrying_runtime: bool,
    has_stale_runtime: bool,
    has_duplicate_runtime: bool,
}

fn summarize_channel_surface_runtime_flags(
    surface: &app::channel::ChannelSurface,
) -> ChannelSurfaceRuntimeFlags {
    ChannelSurfaceRuntimeFlags {
        has_retrying_runtime: surface
            .configured_accounts
            .iter()
            .any(channel_account_has_retrying_runtime),
        has_stale_runtime: surface
            .configured_accounts
            .iter()
            .any(channel_account_has_stale_runtime),
        has_duplicate_runtime: surface
            .configured_accounts
            .iter()
            .any(channel_account_has_duplicate_runtime),
    }
}

fn runtime_attention_reasons_from_flags(flags: &ChannelSurfaceRuntimeFlags) -> Vec<String> {
    let mut reasons = Vec::new();
    if flags.has_retrying_runtime {
        reasons.push("retrying".to_owned());
    }
    if flags.has_stale_runtime {
        reasons.push("stale".to_owned());
    }
    if flags.has_duplicate_runtime {
        reasons.push("duplicate_runtime_instances".to_owned());
    }
    reasons
}

fn collect_channel_surface_preferred_runtime_owner_pids(
    surface: &app::channel::ChannelSurface,
) -> Vec<u32> {
    let runtime_rollups = summarize_channel_surface_duplicate_runtime_owners(surface);
    runtime_rollups.preferred_runtime_owner_pids
}

fn collect_channel_surface_duplicate_runtime_cleanup_owner_pids(
    surface: &app::channel::ChannelSurface,
) -> Vec<u32> {
    let runtime_rollups = summarize_channel_surface_duplicate_runtime_owners(surface);
    runtime_rollups.duplicate_runtime_cleanup_owner_pids
}

fn collect_channel_surface_last_duplicate_runtime_auto_reclaim_at(
    surface: &app::channel::ChannelSurface,
) -> Option<u64> {
    let cleanup_rollup = summarize_last_duplicate_runtime_cleanup(surface);
    cleanup_rollup.last_duplicate_runtime_auto_reclaim_at
}

fn collect_channel_surface_last_duplicate_runtime_auto_cleanup_owner_pids(
    surface: &app::channel::ChannelSurface,
) -> Vec<u32> {
    let cleanup_rollup = summarize_last_duplicate_runtime_cleanup(surface);
    cleanup_rollup.last_duplicate_runtime_auto_cleanup_owner_pids
}

struct ChannelSurfaceDuplicateRuntimeOwners {
    preferred_runtime_owner_pids: Vec<u32>,
    duplicate_runtime_cleanup_owner_pids: Vec<u32>,
}

fn summarize_channel_surface_duplicate_runtime_owners(
    surface: &app::channel::ChannelSurface,
) -> ChannelSurfaceDuplicateRuntimeOwners {
    let mut preferred_owner_pids = BTreeSet::new();
    let mut cleanup_owner_pids = BTreeSet::new();

    for account in &surface.configured_accounts {
        let Some(runtime) = channel_account_serve_runtime(account) else {
            continue;
        };
        if runtime.duplicate_owner_pids.is_empty() {
            continue;
        }
        if let Some(pid) = runtime.pid {
            preferred_owner_pids.insert(pid);
        }
        let preferred_pid = runtime.pid;
        for owner_pid in &runtime.duplicate_owner_pids {
            if Some(*owner_pid) == preferred_pid {
                continue;
            }
            cleanup_owner_pids.insert(*owner_pid);
        }
    }

    ChannelSurfaceDuplicateRuntimeOwners {
        preferred_runtime_owner_pids: preferred_owner_pids.into_iter().collect(),
        duplicate_runtime_cleanup_owner_pids: cleanup_owner_pids.into_iter().collect(),
    }
}

struct ChannelSurfaceDuplicateRuntimeCleanup {
    last_duplicate_runtime_auto_reclaim_at: Option<u64>,
    last_duplicate_runtime_auto_cleanup_owner_pids: Vec<u32>,
}

fn summarize_last_duplicate_runtime_cleanup(
    surface: &app::channel::ChannelSurface,
) -> ChannelSurfaceDuplicateRuntimeCleanup {
    let last_duplicate_runtime_auto_reclaim_at = surface
        .configured_accounts
        .iter()
        .filter_map(channel_account_serve_runtime)
        .filter_map(|runtime| runtime.last_duplicate_reclaim_at)
        .max();

    let Some(latest_reclaim_at) = last_duplicate_runtime_auto_reclaim_at else {
        return ChannelSurfaceDuplicateRuntimeCleanup {
            last_duplicate_runtime_auto_reclaim_at: None,
            last_duplicate_runtime_auto_cleanup_owner_pids: Vec::new(),
        };
    };

    let mut owner_pids = BTreeSet::new();
    for runtime in surface
        .configured_accounts
        .iter()
        .filter_map(channel_account_serve_runtime)
        .filter(|runtime| runtime.last_duplicate_reclaim_at == Some(latest_reclaim_at))
    {
        for owner_pid in &runtime.last_duplicate_reclaim_cleanup_owner_pids {
            owner_pids.insert(*owner_pid);
        }
    }

    ChannelSurfaceDuplicateRuntimeCleanup {
        last_duplicate_runtime_auto_reclaim_at: Some(latest_reclaim_at),
        last_duplicate_runtime_auto_cleanup_owner_pids: owner_pids.into_iter().collect(),
    }
}

fn collect_channel_surface_recent_runtime_incidents(
    surface: &app::channel::ChannelSurface,
) -> Vec<GatewayOperatorRuntimeIncidentReadModel> {
    let mut incidents = surface
        .configured_accounts
        .iter()
        .filter_map(channel_account_serve_runtime)
        .map(build_runtime_incident_read_models)
        .flatten()
        .collect::<Vec<_>>();

    incidents.sort_by_key(|incident| std::cmp::Reverse(incident.at_ms));
    incidents.truncate(5);
    incidents
}

fn build_runtime_incident_read_models(
    runtime: &app::channel::ChannelOperationRuntime,
) -> Vec<GatewayOperatorRuntimeIncidentReadModel> {
    runtime
        .recent_incidents
        .iter()
        .map(|incident| GatewayOperatorRuntimeIncidentReadModel {
            account_id: runtime.account_id.clone(),
            account_label: runtime.account_label.clone(),
            kind: runtime_incident_kind_text(incident.kind).to_owned(),
            at_ms: incident.at_ms,
            detail: incident.detail.clone(),
            owner_pids: incident.owner_pids.clone(),
        })
        .collect()
}

fn runtime_incident_kind_text(
    kind: app::channel::ChannelOperationRuntimeIncidentKind,
) -> &'static str {
    match kind {
        app::channel::ChannelOperationRuntimeIncidentKind::Failure => "failure",
        app::channel::ChannelOperationRuntimeIncidentKind::Recovery => "recovery",
        app::channel::ChannelOperationRuntimeIncidentKind::DuplicateReclaim => {
            "duplicate_reclaim"
        }
    }
}

fn json_string_field(value: &Value, field: &str) -> Option<String> {
    let object = value.as_object()?;
    let value = object.get(field)?;
    let text = value.as_str()?;
    Some(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RUNTIME_TOOL_ACCESS_SEPARATION_NOTE, app};
    use crate::gateway::read_models::{
        GatewayChannelInventoryReadModel, GatewayChannelInventorySchema,
        GatewayChannelInventorySummaryReadModel, GatewayChannelOperationalModelCountsReadModel,
        GatewayChannelRuntimeKindCountsReadModel,
        GatewayChannelServiceContractModelCountsReadModel, GatewayRuntimeSnapshotChannelsReadModel,
        GatewayRuntimeSnapshotReadModel, GatewayRuntimeSnapshotSchema,
        GatewayRuntimeSnapshotToolsReadModel, GatewayToolAccessReadModel,
        GatewayToolCallingReadModel, GatewayToolSurfaceReadModel,
    };

    #[test]
    fn operator_channel_surface_read_model_keeps_plugin_backed_summary_context() {
        let config: app::config::LoongConfig = serde_json::from_value(serde_json::json!({
            "weixin": {
                "enabled": true,
                "default_account": "ops",
                "accounts": {
                    "ops": {
                        "enabled": true,
                        "bridge_url": "https://bridge.example.test/ops",
                        "bridge_access_token": "ops-token",
                        "allowed_contact_ids": ["wxid_ops"]
                    },
                    "backup": {
                        "enabled": true,
                        "bridge_access_token": "backup-token",
                        "allowed_contact_ids": ["wxid_backup"]
                    }
                }
            }
        }))
        .expect("deserialize weixin config");
        let inventory = app::channel::channel_inventory(&config);
        let surface = inventory
            .channel_surfaces
            .iter()
            .find(|surface| surface.catalog.id == "weixin")
            .expect("weixin surface");
        let read_model = core::build_channel_surface_read_model(surface.clone());
        let operator_surface = build_operator_channel_surface_read_model(
            &read_model,
            &inventory.channel_access_policies,
            &Vec::new(),
        );

        assert_eq!(operator_surface.channel_id, "weixin");
        assert_eq!(operator_surface.implementation_status, "plugin_backed");
        assert_eq!(
            operator_surface.service_contract_model,
            "external_plugin_bridge"
        );
        assert_eq!(operator_surface.conversation_gated_account_count, 0);
        assert_eq!(operator_surface.sender_gated_account_count, 0);
        assert_eq!(operator_surface.runtime_attention_account_count, 0);
        assert!(operator_surface.runtime_attention_reasons.is_empty());
        assert!(operator_surface.runtime_attention_remediations.is_empty());
        assert_eq!(operator_surface.retrying_runtime_account_count, 0);
        assert_eq!(
            operator_surface.plugin_bridge_account_summary.as_deref(),
            Some(
                "configured_account=ops (default): ready; configured_account=backup: bridge_url is missing"
            )
        );
    }

    #[test]
    fn operator_channel_surface_read_model_keeps_plugin_bridge_summary_empty_without_discovery_context()
    {
        let mut config = app::config::LoongConfig::default();
        config.telegram.enabled = true;
        config.telegram.bot_token = Some(loong_contracts::SecretRef::Inline(
            "123456:test-token".to_owned(),
        ));
        config.telegram.allowed_chat_ids = vec![1];
        let inventory = app::channel::channel_inventory(&config);
        let surface = inventory
            .channel_surfaces
            .iter()
            .find(|surface| surface.catalog.id == "telegram")
            .expect("telegram surface");
        let read_model = core::build_channel_surface_read_model(surface.clone());
        let operator_surface = build_operator_channel_surface_read_model(
            &read_model,
            &inventory.channel_access_policies,
            &Vec::new(),
        );

        assert_eq!(operator_surface.channel_id, "telegram");
        assert_eq!(operator_surface.implementation_status, "plugin_backed");
        assert_eq!(
            operator_surface.service_contract_model,
            "managed_bridge_capable_service"
        );
        assert_eq!(operator_surface.conversation_gated_account_count, 1);
        assert_eq!(operator_surface.sender_gated_account_count, 0);
        assert_eq!(operator_surface.runtime_attention_account_count, 0);
        assert!(operator_surface.runtime_attention_reasons.is_empty());
        assert!(operator_surface.runtime_attention_remediations.is_empty());
        assert_eq!(operator_surface.plugin_bridge_account_summary, None);
    }

    #[test]
    fn operator_channel_surface_read_model_counts_retrying_runtime_attention() {
        let config = app::config::LoongConfig::default();
        let mut inventory = app::channel::channel_inventory(&config);
        let surface = inventory
            .channel_surfaces
            .iter_mut()
            .find(|surface| surface.catalog.id == "weixin")
            .expect("weixin surface");
        let account = surface
            .configured_accounts
            .iter_mut()
            .find(|account| account.configured_account_id == "default")
            .expect("default weixin account");
        let serve = account
            .operations
            .iter_mut()
            .find(|operation| operation.id == "serve")
            .expect("weixin serve operation");
        serve.runtime = Some(app::channel::ChannelOperationRuntime {
            running: true,
            stale: false,
            busy: false,
            active_runs: 0,
            consecutive_failures: 2,
            last_run_activity_at: Some(1_700_000_000_000),
            last_heartbeat_at: Some(1_700_000_005_000),
            last_failure_at: Some(1_700_000_006_000),
            last_recovery_at: None,
            last_error: Some("temporary bridge timeout".to_owned()),
            last_duplicate_reclaim_at: None,
            pid: Some(5151),
            account_id: Some("default".to_owned()),
            account_label: Some("default".to_owned()),
            instance_count: 1,
            running_instances: 1,
            stale_instances: 0,
            duplicate_owner_pids: Vec::new(),
            last_duplicate_reclaim_cleanup_owner_pids: Vec::new(),
            recent_incidents: Vec::new(),
        });

        let read_model = core::build_channel_surface_read_model(surface.clone());
        let operator_surface = build_operator_channel_surface_read_model(
            &read_model,
            &inventory.channel_access_policies,
            &["weixin".to_owned()],
        );

        assert_eq!(operator_surface.runtime_attention_account_count, 1);
        assert_eq!(
            operator_surface.runtime_attention_reasons,
            vec!["retrying".to_owned()]
        );
        assert_eq!(
            operator_surface.runtime_attention_remediations,
            vec!["inspect_bridge_connectivity".to_owned()]
        );
        assert_eq!(operator_surface.retrying_runtime_account_count, 1);
        assert_eq!(operator_surface.stale_runtime_account_count, 0);
        assert_eq!(operator_surface.duplicate_runtime_account_count, 0);
        assert!(operator_surface.preferred_runtime_owner_pids.is_empty());
        assert!(
            operator_surface
                .duplicate_runtime_cleanup_owner_pids
                .is_empty()
        );
        assert!(
            operator_surface
                .last_duplicate_runtime_auto_reclaim_at
                .is_none()
        );
        assert!(
            operator_surface
                .last_duplicate_runtime_auto_cleanup_owner_pids
                .is_empty()
        );
        assert!(operator_surface.service_enabled);
        assert!(!operator_surface.service_ready);
    }

    #[test]
    fn operator_channel_surface_read_model_collects_duplicate_runtime_owner_pids() {
        let config = app::config::LoongConfig::default();
        let mut inventory = app::channel::channel_inventory(&config);
        let surface = inventory
            .channel_surfaces
            .iter_mut()
            .find(|surface| surface.catalog.id == "weixin")
            .expect("weixin surface");
        let account = surface
            .configured_accounts
            .iter_mut()
            .find(|account| account.configured_account_id == "default")
            .expect("default weixin account");
        let serve = account
            .operations
            .iter_mut()
            .find(|operation| operation.id == "serve")
            .expect("weixin serve operation");
        serve.runtime = Some(app::channel::ChannelOperationRuntime {
            running: true,
            stale: false,
            busy: false,
            active_runs: 0,
            consecutive_failures: 0,
            last_run_activity_at: Some(1_700_000_000_000),
            last_heartbeat_at: Some(1_700_000_005_000),
            last_failure_at: None,
            last_recovery_at: None,
            last_error: None,
            last_duplicate_reclaim_at: Some(1_700_000_007_000),
            pid: Some(6262),
            account_id: Some("default".to_owned()),
            account_label: Some("default".to_owned()),
            instance_count: 2,
            running_instances: 2,
            stale_instances: 0,
            duplicate_owner_pids: vec![5151, 6262],
            last_duplicate_reclaim_cleanup_owner_pids: vec![5151],
            recent_incidents: vec![app::channel::ChannelOperationRuntimeIncident {
                at_ms: 1_700_000_007_000,
                kind: app::channel::ChannelOperationRuntimeIncidentKind::DuplicateReclaim,
                detail: Some(
                    "requested cooperative shutdown for duplicate runtime owners".to_owned(),
                ),
                owner_pids: vec![5151],
            }],
        });

        let read_model = core::build_channel_surface_read_model(surface.clone());
        let operator_surface = build_operator_channel_surface_read_model(
            &read_model,
            &inventory.channel_access_policies,
            &["weixin".to_owned()],
        );

        assert_eq!(operator_surface.runtime_attention_account_count, 1);
        assert_eq!(
            operator_surface.runtime_attention_reasons,
            vec!["duplicate_runtime_instances".to_owned()]
        );
        assert_eq!(
            operator_surface.runtime_attention_remediations,
            vec!["stop_duplicate_runtime_instances".to_owned()]
        );
        assert_eq!(operator_surface.duplicate_runtime_account_count, 1);
        assert_eq!(operator_surface.preferred_runtime_owner_pids, vec![6262]);
        assert_eq!(
            operator_surface.duplicate_runtime_cleanup_owner_pids,
            vec![5151]
        );
        assert_eq!(
            operator_surface.last_duplicate_runtime_auto_reclaim_at,
            Some(1_700_000_007_000)
        );
        assert_eq!(
            operator_surface.last_duplicate_runtime_auto_cleanup_owner_pids,
            vec![5151]
        );
        assert_eq!(operator_surface.recent_runtime_incidents.len(), 1);
        assert_eq!(
            operator_surface.recent_runtime_incidents[0].kind,
            "duplicate_reclaim"
        );
        assert_eq!(
            operator_surface.recent_runtime_incidents[0].owner_pids,
            vec![5151]
        );
        assert!(!operator_surface.service_ready);
    }

    #[test]
    fn operator_channels_summary_read_model_collects_runtime_attention_surface_ids() {
        let config = app::config::LoongConfig::default();
        let mut inventory = app::channel::channel_inventory(&config);
        let surface = inventory
            .channel_surfaces
            .iter_mut()
            .find(|surface| surface.catalog.id == "weixin")
            .expect("weixin surface");
        let account = surface
            .configured_accounts
            .iter_mut()
            .find(|account| account.configured_account_id == "default")
            .expect("default weixin account");
        let serve = account
            .operations
            .iter_mut()
            .find(|operation| operation.id == "serve")
            .expect("weixin serve operation");
        serve.runtime = Some(app::channel::ChannelOperationRuntime {
            running: true,
            stale: false,
            busy: false,
            active_runs: 0,
            consecutive_failures: 2,
            last_run_activity_at: Some(1_700_000_000_000),
            last_heartbeat_at: Some(1_700_000_005_000),
            last_failure_at: Some(1_700_000_006_000),
            last_recovery_at: None,
            last_error: Some("temporary bridge timeout".to_owned()),
            last_duplicate_reclaim_at: None,
            pid: Some(5151),
            account_id: Some("default".to_owned()),
            account_label: Some("default".to_owned()),
            instance_count: 1,
            running_instances: 1,
            stale_instances: 0,
            duplicate_owner_pids: Vec::new(),
            last_duplicate_reclaim_cleanup_owner_pids: Vec::new(),
            recent_incidents: Vec::new(),
        });

        let channel_inventory = core::build_channel_inventory_read_model("/tmp/loong.toml", &inventory);
        let runtime_snapshot = GatewayRuntimeSnapshotReadModel {
            config: "/tmp/loong.toml".to_owned(),
            schema: GatewayRuntimeSnapshotSchema {
                version: 1,
                surface: "runtime_snapshot",
                purpose: "test",
            },
            provider: serde_json::json!({}),
            context_engine: serde_json::json!({}),
            memory_system: serde_json::json!({}),
            acp: serde_json::json!({}),
            channels: GatewayRuntimeSnapshotChannelsReadModel {
                enabled_channel_ids: vec!["weixin".to_owned()],
                enabled_runtime_backed_channel_ids: Vec::new(),
                enabled_service_channel_ids: vec!["weixin".to_owned()],
                enabled_plugin_backed_channel_ids: vec!["weixin".to_owned()],
                enabled_outbound_only_channel_ids: Vec::new(),
                inventory: channel_inventory.clone(),
            },
            tool_runtime: serde_json::json!({}),
            tools: GatewayRuntimeSnapshotToolsReadModel {
                visible_tool_count: 0,
                visible_tool_names: Vec::new(),
                visible_direct_tool_names: Vec::new(),
                hidden_tool_count: 0,
                hidden_tool_tags: Vec::new(),
                hidden_tool_surfaces: Vec::new(),
                capability_snapshot_sha256: "abc123".to_owned(),
                capability_snapshot: "{}".to_owned(),
                tool_calling: GatewayToolCallingReadModel {
                    availability: "ready".to_owned(),
                    structured_tool_schema_enabled: true,
                    effective_tool_schema_mode: "enabled".to_owned(),
                    active_model: "gpt-4.1-mini".to_owned(),
                    reason: "test".to_owned(),
                },
                access: GatewayToolAccessReadModel {
                    ordinary_network_access_enabled: false,
                    query_search_enabled: false,
                    query_search_default_provider: "duckduckgo".to_owned(),
                    query_search_source: "external_provider".to_owned(),
                    query_search_provider_label: "DuckDuckGo".to_owned(),
                    query_search_credential_ready: false,
                    browser_page_access_enabled: false,
                    managed_browser_session_enabled: false,
                    managed_browser_session_ready: false,
                    consent_mode: "full".to_owned(),
                    approval_mode: "disabled".to_owned(),
                    separation_note: RUNTIME_TOOL_ACCESS_SEPARATION_NOTE.to_owned(),
                },
            },
            runtime_plugins: serde_json::json!({}),
            skills: serde_json::json!({}),
        };

        let summary =
            build_operator_channels_summary_read_model(&channel_inventory, &runtime_snapshot);

        assert_eq!(
            summary.runtime_attention_surface_ids,
            vec!["weixin".to_owned()]
        );
        assert_eq!(
            summary.retrying_runtime_surface_ids,
            vec!["weixin".to_owned()]
        );
        assert!(summary.stale_runtime_surface_ids.is_empty());
        assert!(summary.duplicate_runtime_surface_ids.is_empty());
    }

    #[test]
    fn operator_runtime_summary_includes_hidden_surface_ids() {
        let runtime_snapshot = GatewayRuntimeSnapshotReadModel {
            config: "/tmp/loongclaw.toml".to_owned(),
            schema: GatewayRuntimeSnapshotSchema {
                version: 1,
                surface: "runtime_snapshot",
                purpose: "runtime_introspection",
            },
            provider: serde_json::json!({
                "active_profile_id": "demo",
                "active_label": "Demo"
            }),
            context_engine: serde_json::json!({}),
            memory_system: serde_json::json!({}),
            acp: serde_json::json!({}),
            channels: GatewayRuntimeSnapshotChannelsReadModel {
                enabled_channel_ids: vec![],
                enabled_runtime_backed_channel_ids: vec![],
                enabled_service_channel_ids: vec![],
                enabled_plugin_backed_channel_ids: vec![],
                enabled_outbound_only_channel_ids: vec![],
                inventory: GatewayChannelInventoryReadModel {
                    config: "/tmp/loongclaw.toml".to_owned(),
                    schema: GatewayChannelInventorySchema {
                        version: 1,
                        primary_channel_view: "channel_surfaces",
                        catalog_view: "channel_catalog",
                        legacy_channel_views: &[],
                    },
                    summary: GatewayChannelInventorySummaryReadModel {
                        total_surface_count: 0,
                        runtime_backed_surface_count: 0,
                        config_backed_surface_count: 0,
                        plugin_backed_surface_count: 0,
                        catalog_only_surface_count: 0,
                        runtime_kind_counts: GatewayChannelRuntimeKindCountsReadModel {
                            runtime_backed: 0,
                            plugin_backed: 0,
                            outbound_only: 0,
                            catalog_only: 0,
                        },
                        operational_model_counts: GatewayChannelOperationalModelCountsReadModel {
                            gateway_supervised: 0,
                            standalone_runtime: 0,
                            plugin_backed: 0,
                            outbound_only: 0,
                            catalog_only: 0,
                        },
                        service_contract_model_counts:
                            GatewayChannelServiceContractModelCountsReadModel {
                                managed_bridge_capable_service: 0,
                                native_service_channel: 0,
                                standalone_native_service: 0,
                                external_plugin_bridge: 0,
                                direct_send_only: 0,
                                catalog_only: 0,
                            },
                    },
                    channels: vec![],
                    catalog_only_channels: vec![],
                    channel_catalog: vec![],
                    channel_surfaces: vec![],
                    channel_access_policies: vec![],
                },
            },
            tool_runtime: serde_json::json!({}),
            tools: GatewayRuntimeSnapshotToolsReadModel {
                visible_tool_count: 3,
                visible_tool_names: vec!["tool.search".to_owned()],
                visible_direct_tool_names: vec!["read".to_owned(), "write".to_owned()],
                hidden_tool_count: 2,
                hidden_tool_tags: vec!["session".to_owned(), "web".to_owned()],
                hidden_tool_surfaces: vec![
                    GatewayToolSurfaceReadModel {
                        surface_id: "agent".to_owned(),
                        prompt_snippet: "inspect agent runtime state".to_owned(),
                        usage_guidance: "use for approvals, sessions, routing, or delegation"
                            .to_owned(),
                        tool_count: 2,
                        visible_tool_names: vec![
                            "session_events".to_owned(),
                            "session_status".to_owned(),
                        ],
                        tool_ids: vec!["session_events".to_owned(), "session_status".to_owned()],
                    },
                    GatewayToolSurfaceReadModel {
                        surface_id: "web".to_owned(),
                        prompt_snippet: "hidden http operations".to_owned(),
                        usage_guidance: "use direct web first".to_owned(),
                        tool_count: 1,
                        visible_tool_names: vec!["http.request".to_owned()],
                        tool_ids: vec!["http.request".to_owned()],
                    },
                ],
                capability_snapshot_sha256: "abc123".to_owned(),
                capability_snapshot: String::new(),
                tool_calling: GatewayToolCallingReadModel {
                    availability: "ready".to_owned(),
                    structured_tool_schema_enabled: true,
                    effective_tool_schema_mode: "enabled_with_downgrade".to_owned(),
                    active_model: "gpt-4.1-mini".to_owned(),
                    reason: "runtime ready".to_owned(),
                },
                access: GatewayToolAccessReadModel {
                    ordinary_network_access_enabled: true,
                    query_search_enabled: false,
                    query_search_default_provider: "duckduckgo".to_owned(),
                    query_search_source: "external_provider".to_owned(),
                    query_search_provider_label: "DuckDuckGo".to_owned(),
                    query_search_credential_ready: true,
                    browser_page_access_enabled: true,
                    managed_browser_session_enabled: false,
                    managed_browser_session_ready: false,
                    consent_mode: "full".to_owned(),
                    approval_mode: "disabled".to_owned(),
                    separation_note: RUNTIME_TOOL_ACCESS_SEPARATION_NOTE.to_owned(),
                },
            },
            runtime_plugins: serde_json::json!({}),
            skills: serde_json::json!({}),
        };

        let summary = build_operator_runtime_summary_read_model(&runtime_snapshot);

        assert_eq!(summary.visible_direct_tool_names, vec!["read", "write"]);
        assert_eq!(summary.hidden_tool_surface_ids, vec!["agent", "web"]);
        assert!(summary.access.ordinary_network_access_enabled);
        assert!(!summary.access.query_search_enabled);
    }
}

pub(crate) fn build_operator_channel_surface_read_models(
    channel_surfaces: &[super::GatewayChannelSurfaceReadModel],
    channel_access_policies: &[app::channel::ChannelConfiguredAccountAccessPolicy],
    enabled_service_channel_ids: &[String],
) -> Vec<GatewayOperatorChannelSurfaceReadModel> {
    let mut surfaces = Vec::with_capacity(channel_surfaces.len());

    for channel_surface in channel_surfaces {
        let surface = build_operator_channel_surface_read_model(
            channel_surface,
            channel_access_policies,
            enabled_service_channel_ids,
        );
        surfaces.push(surface);
    }

    surfaces
}

pub(crate) fn build_operator_channel_surface_read_model(
    channel_surface: &super::GatewayChannelSurfaceReadModel,
    channel_access_policies: &[app::channel::ChannelConfiguredAccountAccessPolicy],
    enabled_service_channel_ids: &[String],
) -> GatewayOperatorChannelSurfaceReadModel {
    let surface = &channel_surface.surface;
    let channel_id = surface.catalog.id.to_owned();
    let label = surface.catalog.label.to_owned();
    let implementation_status = surface.catalog.implementation_status.as_str().to_owned();
    let runtime_kind = channel_surface.runtime_kind.clone();
    let operational_model = channel_surface.operational_model.clone();
    let service_contract_model = channel_surface.service_contract_model.clone();
    let account_counts = summarize_operator_channel_surface_accounts(surface);
    let access_counts = summarize_operator_channel_surface_access(channel_access_policies, surface);
    let default_configured_account_id = surface.default_configured_account_id.clone();
    let plugin_bridge_account_summary = channel_surface.plugin_bridge_account_summary.clone();
    let runtime_rollups = summarize_operator_channel_surface_runtime(surface);
    let service_enabled = enabled_service_channel_ids.contains(&channel_id);
    let service_ready =
        service_enabled
            && account_counts.ready_serve_account_count > 0
            && runtime_rollups.runtime_attention_account_count == 0;

    GatewayOperatorChannelSurfaceReadModel {
        channel_id,
        label,
        implementation_status,
        runtime_kind,
        operational_model,
        service_contract_model,
        configured_account_count: account_counts.configured_account_count,
        enabled_account_count: account_counts.enabled_account_count,
        misconfigured_account_count: account_counts.misconfigured_account_count,
        ready_send_account_count: account_counts.ready_send_account_count,
        ready_serve_account_count: account_counts.ready_serve_account_count,
        conversation_gated_account_count: access_counts.conversation_gated_account_count,
        sender_gated_account_count: access_counts.sender_gated_account_count,
        mention_gated_account_count: access_counts.mention_gated_account_count,
        default_configured_account_id,
        plugin_bridge_account_summary,
        runtime_attention_account_count: runtime_rollups.runtime_attention_account_count,
        runtime_attention_reasons: runtime_rollups.runtime_attention_reasons,
        runtime_attention_remediations: runtime_rollups.runtime_attention_remediations,
        retrying_runtime_account_count: runtime_rollups.retrying_runtime_account_count,
        stale_runtime_account_count: runtime_rollups.stale_runtime_account_count,
        duplicate_runtime_account_count: runtime_rollups.duplicate_runtime_account_count,
        preferred_runtime_owner_pids: runtime_rollups.preferred_runtime_owner_pids,
        duplicate_runtime_cleanup_owner_pids: runtime_rollups.duplicate_runtime_cleanup_owner_pids,
        last_duplicate_runtime_auto_reclaim_at: runtime_rollups.last_duplicate_runtime_auto_reclaim_at,
        last_duplicate_runtime_auto_cleanup_owner_pids: runtime_rollups
            .last_duplicate_runtime_auto_cleanup_owner_pids,
        recent_runtime_incidents: runtime_rollups.recent_runtime_incidents,
        service_enabled,
        service_ready,
    }
}

struct GatewayOperatorChannelSurfaceAccountCounts {
    configured_account_count: usize,
    enabled_account_count: usize,
    misconfigured_account_count: usize,
    ready_send_account_count: usize,
    ready_serve_account_count: usize,
}

fn summarize_operator_channel_surface_accounts(
    surface: &app::channel::ChannelSurface,
) -> GatewayOperatorChannelSurfaceAccountCounts {
    GatewayOperatorChannelSurfaceAccountCounts {
        configured_account_count: surface.configured_accounts.len(),
        enabled_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| account.enabled)
            .count(),
        misconfigured_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| core::channel_account_is_misconfigured(account))
            .count(),
        ready_send_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| {
                channel_account_operation_is_ready(account, app::channel::CHANNEL_OPERATION_SEND_ID)
            })
            .count(),
        ready_serve_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| {
                channel_account_operation_is_ready(account, app::channel::CHANNEL_OPERATION_SERVE_ID)
            })
            .count(),
    }
}

struct GatewayOperatorChannelSurfaceAccessCounts {
    conversation_gated_account_count: usize,
    sender_gated_account_count: usize,
    mention_gated_account_count: usize,
}

fn summarize_operator_channel_surface_access(
    channel_access_policies: &[app::channel::ChannelConfiguredAccountAccessPolicy],
    surface: &app::channel::ChannelSurface,
) -> GatewayOperatorChannelSurfaceAccessCounts {
    GatewayOperatorChannelSurfaceAccessCounts {
        conversation_gated_account_count: channel_access_policies
            .iter()
            .filter(|policy| policy.channel_id == surface.catalog.id)
            .filter(|policy| {
                policy.summary.conversation_mode
                    != app::channel::ChannelAccessRestrictionMode::Open
            })
            .count(),
        sender_gated_account_count: channel_access_policies
            .iter()
            .filter(|policy| policy.channel_id == surface.catalog.id)
            .filter(|policy| {
                policy.summary.sender_mode != app::channel::ChannelAccessRestrictionMode::Open
            })
            .count(),
        mention_gated_account_count: channel_access_policies
            .iter()
            .filter(|policy| policy.channel_id == surface.catalog.id)
            .filter(|policy| policy.summary.mention_required)
            .count(),
    }
}

struct GatewayOperatorChannelSurfaceRuntimeRollups {
    runtime_attention_account_count: usize,
    runtime_attention_reasons: Vec<String>,
    runtime_attention_remediations: Vec<String>,
    retrying_runtime_account_count: usize,
    stale_runtime_account_count: usize,
    duplicate_runtime_account_count: usize,
    preferred_runtime_owner_pids: Vec<u32>,
    duplicate_runtime_cleanup_owner_pids: Vec<u32>,
    last_duplicate_runtime_auto_reclaim_at: Option<u64>,
    last_duplicate_runtime_auto_cleanup_owner_pids: Vec<u32>,
    recent_runtime_incidents: Vec<GatewayOperatorRuntimeIncidentReadModel>,
}

fn summarize_operator_channel_surface_runtime(
    surface: &app::channel::ChannelSurface,
) -> GatewayOperatorChannelSurfaceRuntimeRollups {
    let runtime_attention_reasons = collect_channel_surface_runtime_attention_reasons(surface);
    GatewayOperatorChannelSurfaceRuntimeRollups {
        runtime_attention_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| channel_account_has_runtime_attention(account))
            .count(),
        runtime_attention_remediations: runtime_attention_reasons
            .iter()
            .map(|reason| runtime_attention_reason_remediation(reason.as_str()).to_owned())
            .collect(),
        retrying_runtime_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| channel_account_has_retrying_runtime(account))
            .count(),
        stale_runtime_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| channel_account_has_stale_runtime(account))
            .count(),
        duplicate_runtime_account_count: surface
            .configured_accounts
            .iter()
            .filter(|account| channel_account_has_duplicate_runtime(account))
            .count(),
        preferred_runtime_owner_pids: collect_channel_surface_preferred_runtime_owner_pids(surface),
        duplicate_runtime_cleanup_owner_pids:
            collect_channel_surface_duplicate_runtime_cleanup_owner_pids(surface),
        last_duplicate_runtime_auto_reclaim_at:
            collect_channel_surface_last_duplicate_runtime_auto_reclaim_at(surface),
        last_duplicate_runtime_auto_cleanup_owner_pids:
            collect_channel_surface_last_duplicate_runtime_auto_cleanup_owner_pids(surface),
        recent_runtime_incidents: collect_channel_surface_recent_runtime_incidents(surface),
        runtime_attention_reasons,
    }
}
