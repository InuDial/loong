use std::net::{Ipv4Addr, SocketAddrV4};

use serde::Serialize;
use serde_json::Value;

use crate::{
    CliResult, build_channels_cli_json_payload,
    collect_runtime_snapshot_cli_state_from_loaded_config, mvp, supervisor::LoadedSupervisorConfig,
};

use super::read_models::{
    GatewayChannelInventoryReadModel, GatewayRuntimeSnapshotReadModel,
    build_runtime_snapshot_read_model,
};
use super::state::GatewayPortSource;

const GATEWAY_ACP_SESSION_LIST_DEFAULT_LIMIT: usize = 50;
const GATEWAY_ACP_SESSION_LIST_MAX_LIMIT: usize = 200;
pub(super) const GATEWAY_CONTROL_PORT_ENV: &str = "LOONG_GATEWAY_PORT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GatewayPortResolution {
    pub(super) port: u16,
    pub(super) source: GatewayPortSource,
}

pub(super) fn build_gateway_channel_inventory_read_model(
    loaded_config: &LoadedSupervisorConfig,
) -> CliResult<GatewayChannelInventoryReadModel> {
    let config_path = loaded_config.resolved_path.display().to_string();
    let inventory = mvp::channel::channel_inventory(&loaded_config.config);
    let read_model = build_channels_cli_json_payload(config_path.as_str(), &inventory);
    Ok(read_model)
}

pub(super) fn build_gateway_runtime_snapshot_read_model(
    loaded_config: &LoadedSupervisorConfig,
) -> CliResult<GatewayRuntimeSnapshotReadModel> {
    let snapshot = collect_runtime_snapshot_cli_state_from_loaded_config(loaded_config)?;
    let read_model = build_runtime_snapshot_read_model(&snapshot);
    Ok(read_model)
}

pub(super) fn gateway_acp_session_list_limit(requested_limit: Option<usize>) -> usize {
    let requested_limit = requested_limit.unwrap_or(GATEWAY_ACP_SESSION_LIST_DEFAULT_LIMIT);
    requested_limit.clamp(1, GATEWAY_ACP_SESSION_LIST_MAX_LIMIT)
}

pub(super) fn sort_gateway_acp_sessions(sessions: &mut [crate::mvp::acp::AcpSessionMetadata]) {
    sessions.sort_by(|left, right| {
        let activity_order = right.last_activity_ms.cmp(&left.last_activity_ms);
        if activity_order == std::cmp::Ordering::Equal {
            return left.session_key.cmp(&right.session_key);
        }
        activity_order
    });
}

pub(super) fn serialize_json_value<T: Serialize>(value: &T, context: &str) -> CliResult<Value> {
    serde_json::to_value(value).map_err(|error| format!("serialize {context} failed: {error}"))
}

fn default_gateway_control_listener_address(config: &mvp::config::LoongConfig) -> SocketAddrV4 {
    SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.gateway.port)
}

pub(super) fn gateway_control_listener_address_from_port_resolution(
    resolution: GatewayPortResolution,
) -> SocketAddrV4 {
    let bind_address = Ipv4Addr::LOCALHOST;
    let bind_port = resolution.port;
    SocketAddrV4::new(bind_address, bind_port)
}

pub(super) fn resolve_gateway_control_listener_port(
    config: &mvp::config::LoongConfig,
    port_override: Option<u16>,
) -> CliResult<GatewayPortResolution> {
    if let Some(port_override) = port_override {
        return Ok(GatewayPortResolution {
            port: port_override,
            source: if port_override == 0 {
                GatewayPortSource::EphemeralCli
            } else {
                GatewayPortSource::Cli
            },
        });
    }

    if let Some(port) = resolve_gateway_control_listener_port_from_env()? {
        return Ok(GatewayPortResolution {
            port,
            source: GatewayPortSource::Env,
        });
    }

    if config.gateway.port != mvp::config::GatewayConfig::default().port {
        return Ok(GatewayPortResolution {
            port: config.gateway.port,
            source: GatewayPortSource::Config,
        });
    }

    let default_port = default_gateway_control_listener_address(config).port();
    Ok(GatewayPortResolution {
        port: default_port,
        source: GatewayPortSource::Default,
    })
}

fn resolve_gateway_control_listener_port_from_env() -> CliResult<Option<u16>> {
    let Some(raw_value) = std::env::var_os(GATEWAY_CONTROL_PORT_ENV) else {
        return Ok(None);
    };
    let raw_value = raw_value.to_string_lossy();
    let trimmed_value = raw_value.trim();
    if trimmed_value.is_empty() {
        return Ok(None);
    }
    let port = trimmed_value.parse::<u16>().map_err(|error| {
        format!("parse {GATEWAY_CONTROL_PORT_ENV}=`{trimmed_value}` failed: {error}")
    })?;
    Ok(Some(port))
}
