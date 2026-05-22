use super::*;

#[test]
fn default_bind_addr_is_loopback() {
    let addr = default_control_plane_bind_addr(0);
    assert_eq!(addr.ip(), Ipv4Addr::LOCALHOST);
    assert_eq!(addr.port(), 0);
}

#[test]
fn resolve_control_plane_bind_addr_accepts_explicit_override() {
    let bind_addr = resolve_control_plane_bind_addr(Some("0.0.0.0:4317"), 0).expect("bind addr");
    assert_eq!(bind_addr, non_loopback_bind_addr());
}

#[test]
fn non_loopback_exposure_requires_explicit_remote_opt_in() {
    let config = mvp::config::LoongConfig::default();
    let error = build_control_plane_exposure_policy(non_loopback_bind_addr(), Some(&config))
        .expect_err("remote bind should require explicit opt-in");
    assert!(error.contains("control_plane.allow_remote=true"));
}

#[test]
fn non_loopback_exposure_requires_shared_token() {
    let mut config = mvp::config::LoongConfig::default();
    config.control_plane.allow_remote = true;
    let error = build_control_plane_exposure_policy(non_loopback_bind_addr(), Some(&config))
        .expect_err("remote bind should require shared token");
    assert!(error.contains("control_plane.shared_token"));
}

#[tokio::test]
async fn readyz_returns_ok() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let router = build_control_plane_router(manager);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("readyz response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn control_challenge_returns_nonce_payload() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let router = build_control_plane_router(manager);
    let challenge = issue_challenge(&router).await;
    assert!(challenge.nonce.starts_with("cpc-"));
    assert!(challenge.expires_at_ms >= challenge.issued_at_ms);
}

#[tokio::test]
async fn healthz_returns_snapshot_json() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    manager.set_presence_count(2);
    manager.set_session_count(3);
    manager.set_pending_approval_count(1);
    manager.set_acp_session_count(4);
    let router = build_control_plane_router(manager);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("healthz response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let snapshot: ControlPlaneSnapshotResponse =
        serde_json::from_slice(&body).expect("snapshot json");
    assert!(snapshot.snapshot.runtime_ready);
    assert_eq!(snapshot.snapshot.presence_count, 2);
    assert_eq!(snapshot.snapshot.session_count, 3);
    assert_eq!(snapshot.snapshot.pending_approval_count, 1);
    assert_eq!(snapshot.snapshot.acp_session_count, 4);
}

#[tokio::test]
async fn control_connect_returns_protocol_response() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: None,
    };

    let response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("connect response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let connect: ControlPlaneConnectResponse = serde_json::from_slice(&body).expect("connect json");
    assert_eq!(connect.protocol, CONTROL_PLANE_PROTOCOL_VERSION);
    assert_eq!(connect.principal.client_id, "cli");
    assert_eq!(connect.principal.role, ControlPlaneRole::Operator);
    assert!(connect.connection_token.starts_with("cpt-"));
    assert!(connect.connection_token_expires_at_ms > 0);
    assert!(connect.snapshot.runtime_ready);
    assert_eq!(
        connect.policy.tick_interval_ms,
        CONTROL_PLANE_TICK_INTERVAL_MS
    );
}

#[tokio::test]
async fn remote_control_connect_requires_shared_token_for_non_device_operator() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_remote_control_plane_router(manager, "bootstrap-token");
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: None,
    };

    let response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("connect response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&body).expect("error json");
    assert_eq!(
        error.code,
        ControlPlaneConnectErrorCode::SharedTokenRequired
    );
}

#[tokio::test]
async fn remote_control_connect_rejects_invalid_shared_token() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_remote_control_plane_router(manager, "bootstrap-token");
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: Some(loong_protocol::ControlPlaneAuthClaims {
            token: Some("wrong-token".to_owned()),
            device_token: None,
            bootstrap_token: None,
            password: None,
        }),
        device: None,
    };

    let response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("connect response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&body).expect("error json");
    assert_eq!(error.code, ControlPlaneConnectErrorCode::SharedTokenInvalid);
}

#[tokio::test]
async fn remote_control_connect_accepts_valid_shared_token() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_remote_control_plane_router(manager, "bootstrap-token");
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: Some(loong_protocol::ControlPlaneAuthClaims {
            token: Some("bootstrap-token".to_owned()),
            device_token: None,
            bootstrap_token: None,
            password: None,
        }),
        device: None,
    };

    let response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("connect response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let connect: ControlPlaneConnectResponse = serde_json::from_slice(&body).expect("connect json");
    assert_eq!(connect.principal.client_id, "cli");
}

#[tokio::test]
async fn remote_control_connect_clamps_bootstrap_scopes_to_safe_subset() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_remote_control_plane_router(manager, "bootstrap-token");
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: std::collections::BTreeSet::from([
            ControlPlaneScope::OperatorRead,
            ControlPlaneScope::OperatorAdmin,
            ControlPlaneScope::OperatorPairing,
        ]),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: Some(loong_protocol::ControlPlaneAuthClaims {
            token: Some("bootstrap-token".to_owned()),
            device_token: None,
            bootstrap_token: None,
            password: None,
        }),
        device: None,
    };

    let response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("connect response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let connect: ControlPlaneConnectResponse = serde_json::from_slice(&body).expect("connect json");
    assert!(
        connect
            .principal
            .scopes
            .contains(&ControlPlaneScope::OperatorRead)
    );
    assert!(
        connect
            .principal
            .scopes
            .contains(&ControlPlaneScope::OperatorPairing)
    );
    assert!(
        !connect
            .principal
            .scopes
            .contains(&ControlPlaneScope::OperatorAdmin)
    );
}

#[tokio::test]
async fn control_connect_rejects_protocol_mismatch() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let router = build_control_plane_router(manager);
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION + 1,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION + 1,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: None,
        },
        role: ControlPlaneRole::Operator,
        scopes: std::collections::BTreeSet::new(),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: None,
    };

    let response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("connect response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn control_connect_requires_pairing_before_signed_device_can_connect() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);
    let challenge = issue_challenge(&router).await;
    let scopes = std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]);
    let device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        scopes.clone(),
        &challenge,
    );
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes,
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: Some(device),
    };

    let response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("connect response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&body).expect("connect error json");
    assert_eq!(error.code, ControlPlaneConnectErrorCode::PairingRequired);
    assert!(error.pairing_request_id.is_some());
}

#[tokio::test]
async fn control_connect_rejects_reused_device_challenge() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);
    let challenge = issue_challenge(&router).await;
    let scopes = std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]);
    let device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        scopes.clone(),
        &challenge,
    );
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes,
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: Some(device),
    };

    let first = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("first connect response");
    assert_eq!(first.status(), StatusCode::FORBIDDEN);

    let second = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("second connect response");
    assert_eq!(second.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pairing_resolve_approves_device_and_connect_accepts_device_token() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);

    let challenge = issue_challenge(&router).await;
    let scopes = std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]);
    let device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        scopes.clone(),
        &challenge,
    );
    let pairing_request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: scopes.clone(),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: Some(device.clone()),
    };

    let pairing_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&pairing_request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("pairing response");
    assert_eq!(pairing_response.status(), StatusCode::FORBIDDEN);
    let pairing_body = to_bytes(pairing_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let pairing_error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&pairing_body).expect("pairing error json");
    let pairing_request_id = pairing_error
        .pairing_request_id
        .expect("pairing request id");

    let operator_token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorPairing]),
    )
    .await;
    let resolve_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/pairing/resolve")
                .method("POST")
                .header("authorization", format!("Bearer {operator_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ControlPlanePairingResolveRequest {
                        pairing_request_id,
                        approve: true,
                    })
                    .expect("encode resolve request"),
                ))
                .expect("request"),
        )
        .await
        .expect("resolve response");
    assert_eq!(resolve_response.status(), StatusCode::OK);
    let resolve_body = to_bytes(resolve_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let resolve: ControlPlanePairingResolveResponse =
        serde_json::from_slice(&resolve_body).expect("resolve json");
    let device_token = resolve.device_token.expect("device token");

    let reconnect_challenge = issue_challenge(&router).await;
    let reconnect_device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        scopes.clone(),
        &reconnect_challenge,
    );
    let reconnect_request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes,
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: Some(loong_protocol::ControlPlaneAuthClaims {
            token: None,
            device_token: Some(device_token),
            bootstrap_token: None,
            password: None,
        }),
        device: Some(reconnect_device),
    };

    let reconnect = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&reconnect_request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("reconnect response");
    assert_eq!(reconnect.status(), StatusCode::OK);
}

#[tokio::test]
async fn control_connect_requires_repairing_for_scope_upgrade() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);

    let initial_scopes = std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]);
    let challenge = issue_challenge(&router).await;
    let device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        initial_scopes.clone(),
        &challenge,
    );
    let initial_request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: initial_scopes.clone(),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: Some(device),
    };

    let pairing_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&initial_request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("pairing response");
    let pairing_body = to_bytes(pairing_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let pairing_error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&pairing_body).expect("pairing error json");
    let pairing_request_id = pairing_error
        .pairing_request_id
        .expect("pairing request id");

    let operator_token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorPairing]),
    )
    .await;
    let resolve_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/pairing/resolve")
                .method("POST")
                .header("authorization", format!("Bearer {operator_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ControlPlanePairingResolveRequest {
                        pairing_request_id,
                        approve: true,
                    })
                    .expect("encode resolve request"),
                ))
                .expect("request"),
        )
        .await
        .expect("resolve response");
    let resolve_body = to_bytes(resolve_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let resolve: ControlPlanePairingResolveResponse =
        serde_json::from_slice(&resolve_body).expect("resolve json");
    let device_token = resolve.device_token.expect("device token");

    let upgraded_scopes = std::collections::BTreeSet::from([
        ControlPlaneScope::OperatorRead,
        ControlPlaneScope::OperatorAcp,
    ]);
    let upgrade_challenge = issue_challenge(&router).await;
    let upgrade_device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        upgraded_scopes.clone(),
        &upgrade_challenge,
    );
    let upgrade_request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: upgraded_scopes,
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: Some(loong_protocol::ControlPlaneAuthClaims {
            token: None,
            device_token: Some(device_token),
            bootstrap_token: None,
            password: None,
        }),
        device: Some(upgrade_device),
    };

    let upgrade_response = router
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&upgrade_request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("upgrade response");
    assert_eq!(upgrade_response.status(), StatusCode::FORBIDDEN);
    let upgrade_body = to_bytes(upgrade_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let upgrade_error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&upgrade_body).expect("upgrade error json");
    assert_eq!(
        upgrade_error.code,
        ControlPlaneConnectErrorCode::PairingRequired
    );
    assert!(upgrade_error.pairing_request_id.is_some());
}

#[tokio::test]
async fn pairing_list_surfaces_pending_request_for_unpaired_device() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);

    let challenge = issue_challenge(&router).await;
    let scopes = std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]);
    let device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        scopes.clone(),
        &challenge,
    );
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes,
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: Some(device),
    };

    let pairing_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("pairing response");
    assert_eq!(pairing_response.status(), StatusCode::FORBIDDEN);

    let operator_token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorPairing]),
    )
    .await;
    let list_response = router
        .oneshot(bearer_request(
            "GET",
            "/pairing/list?status=pending&limit=10",
            &operator_token,
        ))
        .await
        .expect("pairing list response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let list: ControlPlanePairingListResponse =
        serde_json::from_slice(&list_body).expect("pairing list json");
    assert_eq!(list.matched_count, 1);
    assert_eq!(list.returned_count, 1);
    assert_eq!(list.requests[0].status, ControlPlanePairingStatus::Pending);
    assert_eq!(list.requests[0].device_id, "device-1");
}

#[tokio::test]
async fn pairing_list_surfaces_approved_request_after_resolution() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);

    let challenge = issue_challenge(&router).await;
    let scopes = std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]);
    let device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        scopes.clone(),
        &challenge,
    );
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: scopes.clone(),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: Some(device),
    };

    let pairing_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("pairing response");
    assert_eq!(pairing_response.status(), StatusCode::FORBIDDEN);
    let pairing_body = to_bytes(pairing_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let pairing_error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&pairing_body).expect("pairing error json");
    let pairing_request_id = pairing_error
        .pairing_request_id
        .expect("pairing request id");

    let operator_token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorPairing]),
    )
    .await;
    let resolve_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/pairing/resolve")
                .method("POST")
                .header("authorization", format!("Bearer {operator_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ControlPlanePairingResolveRequest {
                        pairing_request_id: pairing_request_id.clone(),
                        approve: true,
                    })
                    .expect("encode resolve request"),
                ))
                .expect("request"),
        )
        .await
        .expect("resolve response");
    assert_eq!(resolve_response.status(), StatusCode::OK);
    let resolve_body = to_bytes(resolve_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let resolve: ControlPlanePairingResolveResponse =
        serde_json::from_slice(&resolve_body).expect("resolve json");
    assert_eq!(resolve.request.status, ControlPlanePairingStatus::Approved,);
    assert_eq!(resolve.request.pairing_request_id, pairing_request_id);
    assert_eq!(resolve.request.requested_scopes, scopes);
    assert!(resolve.request.resolved_at_ms.is_some());
    assert!(resolve.device_token.is_some());

    let list_response = router
        .oneshot(bearer_request(
            "GET",
            "/pairing/list?status=approved&limit=10",
            &operator_token,
        ))
        .await
        .expect("pairing list response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let list: ControlPlanePairingListResponse =
        serde_json::from_slice(&list_body).expect("pairing list json");
    assert_eq!(list.matched_count, 1);
    assert_eq!(list.returned_count, 1);
    assert_eq!(list.requests[0].status, ControlPlanePairingStatus::Approved);
    assert_eq!(list.requests[0].pairing_request_id, pairing_request_id);
    assert_eq!(list.requests[0].requested_scopes, scopes);
    assert!(list.requests[0].resolved_at_ms.is_some());
}

#[tokio::test]
async fn pairing_list_surfaces_rejected_request_after_resolution() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    manager.set_runtime_ready(true);
    let router = build_control_plane_router(manager);

    let challenge = issue_challenge(&router).await;
    let scopes = std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]);
    let device = signed_device_for_request(
        "cli",
        ControlPlaneRole::Operator,
        scopes.clone(),
        &challenge,
    );
    let request = ControlPlaneConnectRequest {
        min_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        max_protocol: CONTROL_PLANE_PROTOCOL_VERSION,
        client: ControlPlaneClientIdentity {
            id: "cli".to_owned(),
            version: "1.0.0".to_owned(),
            mode: "operator_ui".to_owned(),
            platform: "macos".to_owned(),
            display_name: Some("Loong CLI".to_owned()),
        },
        role: ControlPlaneRole::Operator,
        scopes: scopes.clone(),
        caps: std::collections::BTreeSet::new(),
        commands: std::collections::BTreeSet::new(),
        permissions: std::collections::BTreeMap::new(),
        auth: None,
        device: Some(device),
    };

    let pairing_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/control/connect")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode request"),
                ))
                .expect("request"),
        )
        .await
        .expect("pairing response");
    assert_eq!(pairing_response.status(), StatusCode::FORBIDDEN);
    let pairing_body = to_bytes(pairing_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let pairing_error: ControlPlaneConnectErrorResponse =
        serde_json::from_slice(&pairing_body).expect("pairing error json");
    let pairing_request_id = pairing_error
        .pairing_request_id
        .expect("pairing request id");

    let operator_token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorPairing]),
    )
    .await;
    let resolve_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/pairing/resolve")
                .method("POST")
                .header("authorization", format!("Bearer {operator_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ControlPlanePairingResolveRequest {
                        pairing_request_id: pairing_request_id.clone(),
                        approve: false,
                    })
                    .expect("encode resolve request"),
                ))
                .expect("request"),
        )
        .await
        .expect("resolve response");
    assert_eq!(resolve_response.status(), StatusCode::OK);
    let resolve_body = to_bytes(resolve_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let resolve: ControlPlanePairingResolveResponse =
        serde_json::from_slice(&resolve_body).expect("resolve json");
    assert_eq!(resolve.request.status, ControlPlanePairingStatus::Rejected,);
    assert_eq!(resolve.request.pairing_request_id, pairing_request_id);
    assert_eq!(resolve.request.requested_scopes, scopes);
    assert!(resolve.request.resolved_at_ms.is_some());
    assert!(resolve.device_token.is_none());

    let list_response = router
        .oneshot(bearer_request(
            "GET",
            "/pairing/list?status=rejected&limit=10",
            &operator_token,
        ))
        .await
        .expect("pairing list response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let list: ControlPlanePairingListResponse =
        serde_json::from_slice(&list_body).expect("pairing list json");
    assert_eq!(list.matched_count, 1);
    assert_eq!(list.returned_count, 1);
    assert_eq!(list.requests[0].status, ControlPlanePairingStatus::Rejected);
    assert_eq!(list.requests[0].pairing_request_id, pairing_request_id);
    assert_eq!(list.requests[0].requested_scopes, scopes);
    assert!(list.requests[0].resolved_at_ms.is_some());
}

