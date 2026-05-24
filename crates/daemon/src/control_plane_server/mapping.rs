use super::*;

pub(super) fn default_policy() -> ControlPlanePolicy {
    ControlPlanePolicy {
        max_payload_bytes: CONTROL_PLANE_MAX_PAYLOAD_BYTES,
        max_buffered_bytes: CONTROL_PLANE_MAX_BUFFERED_BYTES,
        tick_interval_ms: CONTROL_PLANE_TICK_INTERVAL_MS,
    }
}
