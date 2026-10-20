//! Node → gateway response / unsolicited messages.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use super::{Register, StationId};
use crate::manifest::{AlarmLevel, Quality, SensorUnit};

/// A node reports a telemetry value. Payload of [`MsgType::Telemetry`].
///
/// This is the most common frame on the wire — sensor nodes send these
/// periodically (poll) or on-change (event-driven).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Telemetry {
    pub station_id: StationId,
    pub register: Register,
    pub value: f32,
    pub unit: SensorUnit,
    /// Epoch milliseconds (Unix). 0 if the node has no RTC — the gateway
    /// stamps its receive time in that case.
    pub timestamp_ms: u64,
}

/// A node reports an alarm condition. Payload of [`MsgType::Alarm`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alarm {
    pub station_id: StationId,
    pub register: Register,
    pub level: AlarmLevel,
    /// Human-readable description (e.g. "temperature exceeded HH threshold").
    pub message: String,
    pub timestamp_ms: u64,
}

/// A node reports its lifecycle state. Payload of [`MsgType::Status`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node just booted / joined the bus.
    Boot = 0,
    /// Heartbeat (the node is alive and polling).
    Heartbeat = 1,
    /// Node is shutting down or going to sleep.
    Shutdown = 2,
    /// Node hit an internal error but is still running.
    Degraded = 3,
}

/// Payload of [`MsgType::Status`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    pub station_id: StationId,
    pub state: NodeState,
    /// Optional detail (e.g. firmware version on Boot, error on Degraded).
    pub detail: String,
    pub timestamp_ms: u64,
}

/// A node responds to a [`super::Discover`] probe.
/// Payload of [`MsgType::DiscoverResponse`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoverResponse {
    pub station_id: StationId,
    /// The node's protocol version.
    pub protocol_version: u8,
    /// Station name from the node's manifest (if any).
    pub name: String,
    /// How many registers this station exposes.
    pub register_count: u16,
}

/// Negative acknowledgement. Payload of [`MsgType::Nack`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Nack {
    pub station_id: StationId,
    /// Gateway-defined error code (0 = generic).
    pub error_code: u16,
    pub message: String,
}

/// Register kind of a batch reading — whether the address lives in the
/// coil space (`C:`) or the holding space (`HR:`). Batches are
/// gateway-enriched, so the kind travels with the reading.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RegisterKind {
    /// Holding register (or input register).
    #[default]
    Holding,
    /// Coil (or discrete input).
    Coil,
}

impl RegisterKind {
    /// Canonical addressing prefix (matches the gateway telemetry style:
    /// `HR:{addr}` / `C:{addr}`).
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Holding => "HR",
            Self::Coil => "C",
        }
    }
}

/// One reading inside a [`TelemetryBatch`].
///
/// Unlike the node-originated [`Telemetry`] frame (register + value only,
/// sized for RAM-constrained MCUs), batch readings are gateway-enriched with
/// the semantic name, raw value, and data quality. Batches flow
/// gateway → service, never to MCU nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchReading {
    pub register: Register,
    /// Address space of `register`. Batch payloads are positional (postcard),
    /// so a producer/consumer pair must be upgraded together when this field
    /// changes; the `#[serde(default)]` covers JSON/self-describing formats.
    #[serde(default)]
    pub register_kind: RegisterKind,
    /// Semantic point name from the gateway's manifest (e.g. "pressure_1").
    pub name: String,
    /// Raw producer value (as f64 to cover u16/i16/f32/bool uniformly).
    pub raw: f64,
    /// Scaled engineering value.
    pub value: f64,
    pub unit: SensorUnit,
    /// Data quality of this reading.
    pub quality: Quality,
    /// Epoch milliseconds (Unix).
    pub timestamp_ms: u64,
}

/// A station's enriched telemetry batch. Payload of
/// [`MsgType::TelemetryBatch`].
///
/// Produced by gateways (e.g. evernight) for service consumers (e.g.
/// entelecheia). A full-station batch can exceed the MCU-oriented
/// [`MAX_PAYLOAD_LEN`](super::frame::MAX_PAYLOAD_LEN); links carrying batches
/// should encode/decode with
/// [`MAX_BATCH_PAYLOAD_LEN`](super::frame::MAX_BATCH_PAYLOAD_LEN) headroom.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryBatch {
    pub station_id: StationId,
    /// Epoch milliseconds (Unix) — batch capture time.
    pub timestamp_ms: u64,
    pub readings: Vec<BatchReading>,
}
