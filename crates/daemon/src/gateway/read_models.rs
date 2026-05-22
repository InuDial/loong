pub(crate) mod acp;
pub(crate) mod core;
pub(crate) mod operator;
pub(crate) mod pairing;
pub use acp::*;
pub use core::*;
pub use operator::*;
pub use pairing::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app;

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
        let read_model = build_channel_surface_read_model(surface.clone());
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
        let read_model = build_channel_surface_read_model(surface.clone());
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

        let read_model = build_channel_surface_read_model(surface.clone());
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

        let read_model = build_channel_surface_read_model(surface.clone());
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

        let channel_inventory = build_channel_inventory_read_model("/tmp/loong.toml", &inventory);
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
                    separation_note: crate::RUNTIME_TOOL_ACCESS_SEPARATION_NOTE.to_owned(),
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
    fn channel_inventory_read_model_includes_structured_channel_access_policies() {
        let mut config = app::config::LoongConfig::default();
        config.feishu.enabled = true;
        config.feishu.app_id = Some(loong_contracts::SecretRef::Inline("cli_a1b2c3".to_owned()));
        config.feishu.app_secret = Some(loong_contracts::SecretRef::Inline("secret".to_owned()));
        config.feishu.allowed_chat_ids = vec!["*".to_owned()];
        config.feishu.allowed_sender_ids = vec!["ou_admin".to_owned()];

        let inventory = app::channel::channel_inventory(&config);
        let read_model = build_channel_inventory_read_model("/tmp/loong.toml", &inventory);
        let access_policy = read_model
            .channel_access_policies
            .iter()
            .find(|policy| policy.channel_id == "feishu")
            .expect("feishu access policy");

        assert_eq!(access_policy.conversation_config_key, "allowed_chat_ids");
        assert_eq!(access_policy.sender_config_key, "allowed_sender_ids");
        assert_eq!(
            access_policy.summary.conversation_mode,
            app::channel::ChannelAccessRestrictionMode::WildcardAllowlist
        );
        assert_eq!(
            access_policy.summary.allowed_conversations,
            vec!["*".to_owned()]
        );
        assert_eq!(
            access_policy.summary.allowed_senders,
            vec!["ou_admin".to_owned()]
        );
    }

    #[test]
    fn tool_surface_read_model_preserves_guidance_and_counts() {
        let surface = app::tools::ToolSurfaceState {
            surface_id: "read".to_owned(),
            prompt_snippet: "inspect files".to_owned(),
            usage_guidance: "prefer direct read before shell".to_owned(),
            tool_ids: vec!["file.read".to_owned(), "file.write".to_owned()],
        };

        let read_model = core::build_tool_surface_read_model(&surface);

        assert_eq!(read_model.surface_id, "read");
        assert_eq!(read_model.tool_count, 2);
        assert_eq!(read_model.visible_tool_names, vec!["read", "write"]);
        assert_eq!(read_model.tool_ids, vec!["read", "write"]);
        assert_eq!(read_model.usage_guidance, "prefer direct read before shell");
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
                    separation_note: crate::RUNTIME_TOOL_ACCESS_SEPARATION_NOTE.to_owned(),
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
