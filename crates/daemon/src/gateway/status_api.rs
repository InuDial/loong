use std::sync::Arc;

use axum::{
    extract::State,
    http::HeaderMap,
};

use super::control::{
    GatewayControlAppState, GatewayControlJsonResponse, GatewayControlRequest,
    gateway_control_payload_response, gateway_pairing_registry,
};
use super::read_models::build_gateway_operator_summary_from_registry_read_model;

pub(super) async fn handle_gateway_status(
    headers: HeaderMap,
    State(app_state): State<Arc<GatewayControlAppState>>,
) -> GatewayControlJsonResponse {
    let request = match GatewayControlRequest::authorize(&headers, app_state.as_ref()) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let status = match request.status() {
        Ok(status) => status,
        Err(response) => return response,
    };
    gateway_control_payload_response(&status, "gateway status payload")
}

pub(super) async fn handle_gateway_channels(
    headers: HeaderMap,
    State(app_state): State<Arc<GatewayControlAppState>>,
) -> GatewayControlJsonResponse {
    let request = match GatewayControlRequest::authorize(&headers, app_state.as_ref()) {
        Ok(request) => request,
        Err(response) => return response,
    };
    gateway_control_payload_response(
        request.app_state().channel_inventory.as_ref(),
        "gateway channels payload",
    )
}

pub(super) async fn handle_gateway_runtime_snapshot(
    headers: HeaderMap,
    State(app_state): State<Arc<GatewayControlAppState>>,
) -> GatewayControlJsonResponse {
    let request = match GatewayControlRequest::authorize(&headers, app_state.as_ref()) {
        Ok(request) => request,
        Err(response) => return response,
    };
    gateway_control_payload_response(
        request.app_state().runtime_snapshot.as_ref(),
        "gateway runtime snapshot payload",
    )
}

pub(super) async fn handle_gateway_operator_summary(
    headers: HeaderMap,
    State(app_state): State<Arc<GatewayControlAppState>>,
) -> GatewayControlJsonResponse {
    let request = match GatewayControlRequest::authorize(&headers, app_state.as_ref()) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let status = match request.status() {
        Ok(status) => status,
        Err(response) => return response,
    };
    let pairing_registry = gateway_pairing_registry(request.app_state()).ok();
    let summary = build_gateway_operator_summary_from_registry_read_model(
        &status,
        request.app_state().channel_inventory.as_ref(),
        request.app_state().runtime_snapshot.as_ref(),
        request.app_state().config_path.as_str(),
        pairing_registry.as_ref(),
    );
    gateway_control_payload_response(&summary, "gateway operator summary payload")
}
