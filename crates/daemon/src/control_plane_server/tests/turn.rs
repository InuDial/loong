use super::*;

#[tokio::test]
async fn turn_submit_returns_service_unavailable_without_runtime() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let router = build_control_plane_router(manager);
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorAdmin]),
    )
    .await;
    let request = ControlPlaneTurnSubmitRequest {
        session_id: "session-1".to_owned(),
        input: "hello".to_owned(),
        channel_id: None,
        account_id: None,
        conversation_id: None,
        participant_id: None,
        thread_id: None,
        working_directory: None,
        metadata: std::collections::BTreeMap::new(),
    };
    let response = router
        .oneshot(
            Request::builder()
                .uri("/turn/submit")
                .method("POST")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode turn submit request"),
                ))
                .expect("request"),
        )
        .await
        .expect("turn submit response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn turn_submit_rejects_insufficient_scope() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let state = Arc::new(TestTurnBackendState::default());
    let backend_id: &'static str =
        Box::leak(format!("control-plane-turn-scope-{}", current_time_ms()).into_boxed_str());
    let turn_runtime = seeded_turn_runtime(backend_id, state);
    let router = build_control_plane_router_with_turn_runtime(manager, turn_runtime);
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
    )
    .await;
    let request = ControlPlaneTurnSubmitRequest {
        session_id: "session-1".to_owned(),
        input: "hello".to_owned(),
        channel_id: None,
        account_id: None,
        conversation_id: None,
        participant_id: None,
        thread_id: None,
        working_directory: None,
        metadata: std::collections::BTreeMap::new(),
    };
    let response = router
        .oneshot(
            Request::builder()
                .uri("/turn/submit")
                .method("POST")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode turn submit request"),
                ))
                .expect("request"),
        )
        .await
        .expect("turn submit response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[cfg(feature = "memory-sqlite")]
#[tokio::test]
async fn turn_submit_rejects_hidden_session_visibility() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let backend_state = Arc::new(TestTurnBackendState::default());
    let backend_id: &'static str =
        Box::leak(format!("control-plane-turn-hidden-{}", current_time_ms()).into_boxed_str());
    let turn_runtime = seeded_turn_runtime(backend_id, backend_state);
    let (repository_view, acp_view) = seeded_control_plane_views("turn-hidden-session");
    let router = build_control_plane_router_with_turn_runtime_and_views(
        manager,
        repository_view,
        acp_view,
        turn_runtime,
    );
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorAdmin]),
    )
    .await;
    let request = ControlPlaneTurnSubmitRequest {
        session_id: "hidden-root".to_owned(),
        input: "hello".to_owned(),
        channel_id: None,
        account_id: None,
        conversation_id: None,
        participant_id: None,
        thread_id: None,
        working_directory: None,
        metadata: std::collections::BTreeMap::new(),
    };
    let response = router
        .oneshot(
            Request::builder()
                .uri("/turn/submit")
                .method("POST")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode turn submit request"),
                ))
                .expect("request"),
        )
        .await
        .expect("turn submit response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[cfg(feature = "memory-sqlite")]
#[tokio::test]
async fn turn_result_and_stream_reject_hidden_session_visibility() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let backend_state = Arc::new(TestTurnBackendState::default());
    let backend_id: &'static str = Box::leak(
        format!("control-plane-turn-hidden-result-{}", current_time_ms()).into_boxed_str(),
    );
    let turn_runtime = seeded_turn_runtime(backend_id, backend_state);
    let turn_snapshot = turn_runtime.registry.issue_turn("hidden-root");
    let turn_id = turn_snapshot.turn_id.clone();
    let (repository_view, acp_view) = seeded_control_plane_views("turn-hidden-result");
    let result_router = build_control_plane_router_with_turn_runtime_and_views(
        manager,
        repository_view,
        acp_view,
        turn_runtime.clone(),
    );
    let token = connect_token(
        &result_router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
    )
    .await;
    let result_response = result_router
        .clone()
        .oneshot(bearer_request(
            "GET",
            format!("/turn/result?turn_id={turn_id}").as_str(),
            &token,
        ))
        .await
        .expect("turn result response");
    assert_eq!(result_response.status(), StatusCode::FORBIDDEN);
    let stream_response = result_router
        .oneshot(bearer_request(
            "GET",
            format!("/turn/stream?turn_id={turn_id}").as_str(),
            &token,
        ))
        .await
        .expect("turn stream response");
    assert_eq!(stream_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn turn_submit_and_result_fetch_complete_with_streamed_backend() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let state = Arc::new(TestTurnBackendState::default());
    let backend_id: &'static str =
        Box::leak(format!("control-plane-turn-success-{}", current_time_ms()).into_boxed_str());
    let turn_runtime = seeded_turn_runtime(backend_id, state.clone());
    let router = build_control_plane_router_with_turn_runtime(manager, turn_runtime);
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorAdmin]),
    )
    .await;
    let request = ControlPlaneTurnSubmitRequest {
        session_id: "session-1".to_owned(),
        input: "hello".to_owned(),
        channel_id: None,
        account_id: None,
        conversation_id: None,
        participant_id: None,
        thread_id: None,
        working_directory: None,
        metadata: std::collections::BTreeMap::new(),
    };
    let submit_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/turn/submit")
                .method("POST")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode turn submit request"),
                ))
                .expect("request"),
        )
        .await
        .expect("turn submit response");
    assert_eq!(submit_response.status(), StatusCode::ACCEPTED);
    let submit_body = to_bytes(submit_response.into_body(), usize::MAX)
        .await
        .expect("submit body");
    let submit: ControlPlaneTurnSubmitResponse =
        serde_json::from_slice(&submit_body).expect("submit json");
    assert_eq!(submit.turn.status, ControlPlaneTurnStatus::Running);

    let turn_id = submit.turn.turn_id.clone();
    let mut final_result = None;
    for _ in 0..20 {
        let result_response = router
            .clone()
            .oneshot(bearer_request(
                "GET",
                format!("/turn/result?turn_id={turn_id}").as_str(),
                &token,
            ))
            .await
            .expect("turn result response");
        assert_eq!(result_response.status(), StatusCode::OK);
        let result_body = to_bytes(result_response.into_body(), usize::MAX)
            .await
            .expect("result body");
        let result: ControlPlaneTurnResultResponse =
            serde_json::from_slice(&result_body).expect("result json");
        if result.turn.status.is_terminal() {
            final_result = Some(result);
            break;
        }
        tokio::task::yield_now().await;
    }

    let final_result = final_result.expect("turn should reach a terminal state");
    assert_eq!(
        final_result.turn.status,
        ControlPlaneTurnStatus::Completed,
        "turn result error: {:?}",
        final_result.error
    );
    assert_eq!(final_result.output_text.as_deref(), Some("streamed: hello"));
    assert_eq!(final_result.stop_reason.as_deref(), Some("completed"));
    assert_eq!(
        final_result
            .usage
            .as_ref()
            .and_then(|usage| usage.get("total_tokens")),
        Some(&serde_json::json!(7))
    );
    assert!(
        final_result.turn.event_count >= 3,
        "expected runtime events plus terminal event"
    );
    assert_eq!(
        state.sink_calls.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[tokio::test]
async fn turn_stream_replays_buffered_runtime_and_terminal_events() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let state = Arc::new(TestTurnBackendState::default());
    let backend_id: &'static str =
        Box::leak(format!("control-plane-turn-stream-{}", current_time_ms()).into_boxed_str());
    let turn_runtime = seeded_turn_runtime(backend_id, state);
    let router = build_control_plane_router_with_turn_runtime(manager, turn_runtime);
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorAdmin]),
    )
    .await;
    let request = ControlPlaneTurnSubmitRequest {
        session_id: "session-stream".to_owned(),
        input: "stream me".to_owned(),
        channel_id: None,
        account_id: None,
        conversation_id: None,
        participant_id: None,
        thread_id: None,
        working_directory: None,
        metadata: std::collections::BTreeMap::new(),
    };
    let submit_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/turn/submit")
                .method("POST")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode turn submit request"),
                ))
                .expect("request"),
        )
        .await
        .expect("turn submit response");
    let submit_body = to_bytes(submit_response.into_body(), usize::MAX)
        .await
        .expect("submit body");
    let submit: ControlPlaneTurnSubmitResponse =
        serde_json::from_slice(&submit_body).expect("submit json");
    let turn_id = submit.turn.turn_id;

    for _ in 0..20 {
        let result_response = router
            .clone()
            .oneshot(bearer_request(
                "GET",
                format!("/turn/result?turn_id={turn_id}").as_str(),
                &token,
            ))
            .await
            .expect("turn result response");
        let result_body = to_bytes(result_response.into_body(), usize::MAX)
            .await
            .expect("result body");
        let result: ControlPlaneTurnResultResponse =
            serde_json::from_slice(&result_body).expect("result json");
        if result.turn.status.is_terminal() {
            break;
        }
        tokio::task::yield_now().await;
    }

    let stream_response = router
        .oneshot(bearer_request(
            "GET",
            format!("/turn/stream?turn_id={turn_id}").as_str(),
            &token,
        ))
        .await
        .expect("turn stream response");
    assert_eq!(stream_response.status(), StatusCode::OK);
    let stream_body = to_bytes(stream_response.into_body(), usize::MAX)
        .await
        .expect("stream body");
    let stream_text = String::from_utf8(stream_body.to_vec()).expect("utf8 stream body");
    assert!(stream_text.contains("event: turn.event"));
    assert!(stream_text.contains("event: turn.terminal"));
    assert!(stream_text.contains("\"type\":\"text\""));
    assert!(stream_text.contains("chunk:stream me"));
    assert!(stream_text.contains("\"event_type\":\"turn.completed\""));
}

#[cfg(feature = "memory-sqlite")]
#[tokio::test]
async fn approval_list_rejects_insufficient_scope() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let router = build_control_plane_router_with_views(
        manager,
        Some(seeded_repository_view("approval-list-scope")),
        None,
    );
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
    )
    .await;
    let response = router
        .oneshot(bearer_request(
            "GET",
            "/approval/list?status=pending&limit=10",
            &token,
        ))
        .await
        .expect("approval list response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn turn_submit_preserves_input_whitespace() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let state = Arc::new(TestTurnBackendState::default());
    let backend_id: &'static str =
        Box::leak(format!("control-plane-turn-whitespace-{}", current_time_ms()).into_boxed_str());
    let turn_runtime = seeded_turn_runtime(backend_id, state);
    let router = build_control_plane_router_with_turn_runtime(manager, turn_runtime);
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorAdmin]),
    )
    .await;
    let input = "  hello\n\n```rust\nfn main() {}\n```\n".to_owned();
    let request = ControlPlaneTurnSubmitRequest {
        session_id: "session-whitespace".to_owned(),
        input: input.clone(),
        channel_id: None,
        account_id: None,
        conversation_id: None,
        participant_id: None,
        thread_id: None,
        working_directory: None,
        metadata: std::collections::BTreeMap::new(),
    };
    let submit_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/turn/submit")
                .method("POST")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("encode turn submit request"),
                ))
                .expect("request"),
        )
        .await
        .expect("turn submit response");
    assert_eq!(submit_response.status(), StatusCode::ACCEPTED);
    let submit_body = to_bytes(submit_response.into_body(), usize::MAX)
        .await
        .expect("submit body");
    let submit: ControlPlaneTurnSubmitResponse =
        serde_json::from_slice(&submit_body).expect("submit json");
    let turn_id = submit.turn.turn_id;
    let mut final_result = None;
    for _ in 0..20 {
        let result_response = router
            .clone()
            .oneshot(bearer_request(
                "GET",
                format!("/turn/result?turn_id={turn_id}").as_str(),
                &token,
            ))
            .await
            .expect("turn result response");
        let result_body = to_bytes(result_response.into_body(), usize::MAX)
            .await
            .expect("result body");
        let result: ControlPlaneTurnResultResponse =
            serde_json::from_slice(&result_body).expect("result json");
        if result.turn.status.is_terminal() {
            final_result = Some(result);
            break;
        }
        tokio::task::yield_now().await;
    }
    let final_result = final_result.expect("turn should reach a terminal state");
    let expected_output = format!("streamed: {input}");
    assert_eq!(
        final_result.output_text.as_deref(),
        Some(expected_output.as_str()),
        "turn result error: {:?}",
        final_result.error
    );
}

#[tokio::test]
async fn turn_stream_stops_when_retention_prunes_completed_turn() {
    let registry = Arc::new(mvp::control_plane::ControlPlaneTurnRegistry::new());
    let turn = registry.issue_turn("session-pruned");
    let turn_id = turn.turn_id.clone();
    registry
        .complete_success(turn_id.as_str(), "done", Some("completed"), None)
        .expect("complete pruned turn");
    let initial_state =
        initial_turn_stream_state(registry.clone(), turn_id.as_str(), 1).expect("state");
    for index in 0..300 {
        let session_id = format!("session-retained-{index}");
        let output_text = format!("output-{index}");
        let retained_turn = registry.issue_turn(session_id.as_str());
        registry
            .complete_success(
                retained_turn.turn_id.as_str(),
                output_text.as_str(),
                Some("completed"),
                None,
            )
            .expect("complete retained turn");
    }
    let next_item = next_turn_sse_item(initial_state).await;
    assert!(next_item.is_none());
}

#[cfg(feature = "memory-sqlite")]
#[tokio::test]
async fn acp_session_list_rejects_insufficient_scope() {
    let manager = Arc::new(mvp::control_plane::ControlPlaneManager::new());
    let (_repository_view, acp_view) = seeded_control_plane_views("acp-list-scope");
    let router = build_control_plane_router_with_views(manager, None, Some(acp_view));
    let token = connect_token(
        &router,
        std::collections::BTreeSet::from([ControlPlaneScope::OperatorRead]),
    )
    .await;
    let response = router
        .oneshot(bearer_request("GET", "/acp/session/list?limit=10", &token))
        .await
        .expect("ACP session list response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
