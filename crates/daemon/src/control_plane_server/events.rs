use super::*;

pub(super) fn initial_subscribe_state(
    manager: Arc<mvp::control_plane::ControlPlaneManager>,
    after_seq: u64,
    include_targeted: bool,
) -> ControlPlaneSubscribeStreamState {
    let receiver = manager.subscribe();
    let pending_events = manager.recent_events_after(
        after_seq,
        CONTROL_PLANE_DEFAULT_EVENT_LIMIT,
        include_targeted,
    );
    let pending_events = VecDeque::from(pending_events);
    ControlPlaneSubscribeStreamState {
        manager,
        pending_events,
        receiver,
        last_seq: after_seq,
        include_targeted,
    }
}

pub(super) fn sse_event_from_control_plane_record(
    record: mvp::control_plane::ControlPlaneEventRecord,
) -> Result<Event, String> {
    let seq = record.seq;
    let envelope = map_event(record);
    let event_name = envelope.event.as_str();
    let event_id = seq.to_string();
    let event_builder = Event::default();
    let event_builder = event_builder.event(event_name);
    let event_builder = event_builder.id(event_id);
    event_builder
        .json_data(&envelope)
        .map_err(|error| format!("control-plane SSE event encoding failed: {error}"))
}

pub(super) fn fallback_sse_error_event(message: &str) -> Event {
    let error_message = format!("{{\"error\":\"{message}\"}}");
    let base_event = Event::default();
    let named_event = base_event.event("control.error");
    named_event.data(error_message)
}

pub(super) async fn next_control_plane_sse_item(
    mut state: ControlPlaneSubscribeStreamState,
) -> Option<(Result<Event, Infallible>, ControlPlaneSubscribeStreamState)> {
    if let Some(event) = next_pending_control_plane_sse_item(&mut state) {
        return Some((Ok(event), state));
    }

    receive_next_control_plane_event(&mut state)
        .await
        .map(|event| (Ok(event), state))
}

fn next_pending_control_plane_sse_item(
    state: &mut ControlPlaneSubscribeStreamState,
) -> Option<Event> {
    let record = state.pending_events.pop_front()?;
    state.last_seq = record.seq;
    Some(control_plane_sse_event_or_fallback(record))
}

async fn receive_next_control_plane_event(
    state: &mut ControlPlaneSubscribeStreamState,
) -> Option<Event> {
    loop {
        match state.receiver.recv().await {
            Ok(record) => {
                let include_targeted = state.include_targeted;
                let targeted = record.targeted;
                let already_seen = record.seq <= state.last_seq;
                if (!include_targeted && targeted) || already_seen {
                    continue;
                }
                state.last_seq = record.seq;
                return Some(control_plane_sse_event_or_fallback(record));
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                let refill = state.manager.recent_events_after(
                    state.last_seq,
                    CONTROL_PLANE_DEFAULT_EVENT_LIMIT,
                    state.include_targeted,
                );
                state.pending_events = VecDeque::from(refill);
                if let Some(event) = next_pending_control_plane_sse_item(state) {
                    return Some(event);
                }
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
        }
    }
}

fn control_plane_sse_event_or_fallback(
    record: mvp::control_plane::ControlPlaneEventRecord,
) -> Event {
    match sse_event_from_control_plane_record(record) {
        Ok(event) => event,
        Err(error) => fallback_sse_error_event(error.as_str()),
    }
}

pub(super) fn control_plane_subscribe_stream(
    manager: Arc<mvp::control_plane::ControlPlaneManager>,
    after_seq: u64,
    include_targeted: bool,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let initial_state = initial_subscribe_state(manager, after_seq, include_targeted);
    stream::unfold(initial_state, next_control_plane_sse_item)
}

pub(super) fn map_turn_status(
    status: mvp::control_plane::ControlPlaneTurnStatus,
) -> ControlPlaneTurnStatus {
    match status {
        mvp::control_plane::ControlPlaneTurnStatus::Running => ControlPlaneTurnStatus::Running,
        mvp::control_plane::ControlPlaneTurnStatus::Completed => ControlPlaneTurnStatus::Completed,
        mvp::control_plane::ControlPlaneTurnStatus::Failed => ControlPlaneTurnStatus::Failed,
        mvp::control_plane::ControlPlaneTurnStatus::Cancelled => ControlPlaneTurnStatus::Cancelled,
    }
}

pub(super) fn map_turn_summary(
    snapshot: &mvp::control_plane::ControlPlaneTurnSnapshot,
) -> ControlPlaneTurnSummary {
    ControlPlaneTurnSummary {
        turn_id: snapshot.turn_id.clone(),
        session_id: snapshot.session_id.clone(),
        status: map_turn_status(snapshot.status),
        submitted_at_ms: snapshot.submitted_at_ms,
        completed_at_ms: snapshot.completed_at_ms,
        event_count: snapshot.event_count,
    }
}

pub(super) fn map_turn_result(
    snapshot: &mvp::control_plane::ControlPlaneTurnSnapshot,
) -> ControlPlaneTurnResultResponse {
    ControlPlaneTurnResultResponse {
        turn: map_turn_summary(snapshot),
        output_text: snapshot.output_text.clone(),
        stop_reason: snapshot.stop_reason.clone(),
        usage: snapshot.usage.clone(),
        error: snapshot.error.clone(),
    }
}

pub(super) fn map_turn_event_payload(
    record: &mvp::control_plane::ControlPlaneTurnEventRecord,
) -> serde_json::Value {
    serde_json::json!({
        "turn_id": record.turn_id,
        "session_id": record.session_id,
        "seq": record.seq,
        "terminal": record.terminal,
        "payload": record.payload,
    })
}

pub(super) fn map_turn_event(
    record: mvp::control_plane::ControlPlaneTurnEventRecord,
) -> ControlPlaneTurnEventEnvelope {
    ControlPlaneTurnEventEnvelope {
        turn_id: record.turn_id,
        session_id: record.session_id,
        seq: record.seq,
        terminal: record.terminal,
        payload: record.payload,
    }
}

pub(super) fn sse_event_from_turn_record(
    record: mvp::control_plane::ControlPlaneTurnEventRecord,
) -> Result<Event, String> {
    let seq = record.seq;
    let terminal = record.terminal;
    let envelope = map_turn_event(record);
    let event_name = if terminal {
        "turn.terminal"
    } else {
        "turn.event"
    };
    let event_id = seq.to_string();
    let base_event = Event::default();
    let named_event = base_event.event(event_name);
    let identified_event = named_event.id(event_id);
    identified_event
        .json_data(&envelope)
        .map_err(|error| format!("control-plane turn SSE event encoding failed: {error}"))
}

pub(super) fn fallback_turn_sse_error_event(message: &str) -> Event {
    let error_message = format!("{{\"error\":\"{message}\"}}");
    let base_event = Event::default();
    let named_event = base_event.event("turn.error");
    named_event.data(error_message)
}

pub(super) fn initial_turn_stream_state(
    registry: Arc<mvp::control_plane::ControlPlaneTurnRegistry>,
    turn_id: &str,
    after_seq: u64,
) -> Result<ControlPlaneTurnStreamState, String> {
    let receiver = registry.subscribe();
    let pending_events =
        registry.recent_events_after(turn_id, after_seq, CONTROL_PLANE_DEFAULT_EVENT_LIMIT)?;
    let pending_events = VecDeque::from(pending_events);
    Ok(ControlPlaneTurnStreamState {
        turn_id: turn_id.to_owned(),
        registry,
        pending_events,
        receiver,
        last_seq: after_seq,
    })
}

pub(super) async fn next_turn_sse_item(
    mut state: ControlPlaneTurnStreamState,
) -> Option<(Result<Event, Infallible>, ControlPlaneTurnStreamState)> {
    if let Some(event) = next_pending_turn_sse_item(&mut state) {
        return Some((Ok(event), state));
    }

    if !turn_stream_is_active(&state) {
        return None;
    }

    receive_next_turn_stream_event(&mut state)
        .await
        .map(|event| (Ok(event), state))
}

fn next_pending_turn_sse_item(state: &mut ControlPlaneTurnStreamState) -> Option<Event> {
    let record = state.pending_events.pop_front()?;
    state.last_seq = record.seq;
    Some(turn_sse_event_or_fallback(record))
}

fn turn_stream_is_active(state: &ControlPlaneTurnStreamState) -> bool {
    match state.registry.read_turn(state.turn_id.as_str()) {
        Ok(Some(snapshot)) => !snapshot.status.is_terminal(),
        Ok(None) | Err(_) => false,
    }
}

async fn receive_next_turn_stream_event(state: &mut ControlPlaneTurnStreamState) -> Option<Event> {
    loop {
        match state.receiver.recv().await {
            Ok(record) => {
                if record.turn_id != state.turn_id || record.seq <= state.last_seq {
                    continue;
                }
                state.last_seq = record.seq;
                return Some(turn_sse_event_or_fallback(record));
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                refill_turn_stream_pending_events(state);
                if let Some(event) = next_pending_turn_sse_item(state) {
                    return Some(event);
                }
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
        }
    }
}

fn refill_turn_stream_pending_events(state: &mut ControlPlaneTurnStreamState) {
    let refill_result = state.registry.recent_events_after(
        state.turn_id.as_str(),
        state.last_seq,
        CONTROL_PLANE_DEFAULT_EVENT_LIMIT,
    );
    let refill = refill_result.unwrap_or_default();
    state.pending_events = VecDeque::from(refill);
}

fn turn_sse_event_or_fallback(record: mvp::control_plane::ControlPlaneTurnEventRecord) -> Event {
    match sse_event_from_turn_record(record) {
        Ok(event) => event,
        Err(error) => fallback_turn_sse_error_event(error.as_str()),
    }
}

pub(super) fn control_plane_turn_stream(
    registry: Arc<mvp::control_plane::ControlPlaneTurnRegistry>,
    turn_id: String,
    after_seq: u64,
) -> Result<impl Stream<Item = Result<Event, Infallible>>, String> {
    let initial_state = initial_turn_stream_state(registry, turn_id.as_str(), after_seq)?;
    Ok(stream::unfold(initial_state, next_turn_sse_item))
}
