use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use super::control::{
    GatewayControlAppState, GatewayControlJsonResponse, GatewayControlRequest,
    gateway_control_payload_response, is_gateway_acp_not_found_error,
};
use super::lifecycle::json_error;
use super::read_models::{
    build_acp_observability_read_model, build_acp_session_list_read_model,
    build_acp_status_read_model,
};
use super::support::{gateway_acp_session_list_limit, sort_gateway_acp_sessions};

#[derive(Debug, Default, Deserialize)]
pub(super) struct GatewayAcpSessionsQuery {
    pub(super) limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct GatewayAcpStatusQuery {
    pub(super) session: Option<String>,
    pub(super) conversation_id: Option<String>,
    pub(super) route_session_id: Option<String>,
}

pub(super) async fn handle_gateway_acp_sessions(
    headers: HeaderMap,
    State(app_state): State<Arc<GatewayControlAppState>>,
    Query(query): Query<GatewayAcpSessionsQuery>,
) -> GatewayControlJsonResponse {
    let request = match GatewayControlRequest::authorize(&headers, app_state.as_ref()) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let manager = match request.acp_manager() {
        Ok(manager) => manager,
        Err(response) => return response,
    };

    let sessions_result = manager.list_sessions();
    let mut sessions = match sessions_result {
        Ok(sessions) => sessions,
        Err(error) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "acp_sessions_unavailable",
                error.as_str(),
            );
        }
    };

    sort_gateway_acp_sessions(sessions.as_mut_slice());
    let matched_count = sessions.len();
    let limit = gateway_acp_session_list_limit(query.limit);
    sessions.truncate(limit);

    let payload = build_acp_session_list_read_model(
        request.app_state().config_path.as_str(),
        matched_count,
        sessions.as_slice(),
    );
    gateway_control_payload_response(&payload, "gateway ACP sessions payload")
}

pub(super) async fn handle_gateway_acp_status(
    headers: HeaderMap,
    State(app_state): State<Arc<GatewayControlAppState>>,
    Query(query): Query<GatewayAcpStatusQuery>,
) -> GatewayControlJsonResponse {
    let request = match GatewayControlRequest::authorize(&headers, app_state.as_ref()) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let config = match request.config() {
        Ok(config) => config,
        Err(response) => return response,
    };
    let manager = match request.acp_manager() {
        Ok(manager) => manager,
        Err(response) => return response,
    };

    let resolved_session_key = crate::resolve_acp_status_session_key(
        config,
        query.session.as_deref(),
        query.conversation_id.as_deref(),
        query.route_session_id.as_deref(),
    );
    let resolved_session_key = match resolved_session_key {
        Ok(resolved_session_key) => resolved_session_key,
        Err(error) if is_gateway_acp_not_found_error(error.as_str()) => {
            return json_error(StatusCode::NOT_FOUND, "not_found", error.as_str());
        }
        Err(error) => {
            return json_error(StatusCode::BAD_REQUEST, "invalid_selector", error.as_str());
        }
    };

    let status_result = manager
        .get_status(config, resolved_session_key.as_str())
        .await;
    let status = match status_result {
        Ok(status) => status,
        Err(error) if is_gateway_acp_not_found_error(error.as_str()) => {
            return json_error(StatusCode::NOT_FOUND, "not_found", error.as_str());
        }
        Err(error) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "acp_status_unavailable",
                error.as_str(),
            );
        }
    };

    let payload = build_acp_status_read_model(
        request.app_state().config_path.as_str(),
        query.session.as_deref(),
        query.conversation_id.as_deref(),
        query.route_session_id.as_deref(),
        resolved_session_key.as_str(),
        &status,
    );
    gateway_control_payload_response(&payload, "gateway ACP status payload")
}

pub(super) async fn handle_gateway_acp_observability(
    headers: HeaderMap,
    State(app_state): State<Arc<GatewayControlAppState>>,
) -> GatewayControlJsonResponse {
    let request = match GatewayControlRequest::authorize(&headers, app_state.as_ref()) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let config = match request.config() {
        Ok(config) => config,
        Err(response) => return response,
    };
    let manager = match request.acp_manager() {
        Ok(manager) => manager,
        Err(response) => return response,
    };

    let snapshot_result = manager.observability_snapshot(config).await;
    let snapshot = match snapshot_result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "acp_observability_unavailable",
                error.as_str(),
            );
        }
    };

    let payload =
        build_acp_observability_read_model(request.app_state().config_path.as_str(), &snapshot);
    gateway_control_payload_response(&payload, "gateway ACP observability payload")
}
