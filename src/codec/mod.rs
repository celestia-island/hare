//! Protocol type identifiers and opaque frame containers.
//!
//! This module carries the **type vocabulary only**:
//!
//! - [`ProtocolKind`] — identifies which industrial protocol a station uses.
//! - [`ProtocolFrame`] — an opaque tagged byte buffer for passing raw
//!   protocol data through the wire layer without hare needing to understand it.
//!
//! The family's own codecs (Modbus, Mitsubishi MC Protocol, S7comm) live in
//! [`crate::protocol`]; protocols that need a full third-party client stack
//! (EtherNet/IP-CIP, OPC UA, …) stay in **evernight**, the gateway broker.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Identifies an industrial protocol family.
///
/// Used in manifest `Connection` entries and wire `DiscoverResponse` to
/// declare what protocol a station speaks. The codecs for the family-owned
/// protocols live in [`crate::protocol`].
/// these — evernight owns the full codec stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ProtocolKind {
    /// Modbus RTU (serial RS-485).
    ModbusRtu,
    /// Modbus TCP.
    ModbusTcp,
    /// Siemens S7comm.
    S7comm,
    /// Mitsubishi MC Protocol (3E frame).
    McProtocol,
    /// Rockwell EtherNet/IP + CIP.
    EthernetIp,
    /// EtherCAT.
    Ethercat,
    /// CAN bus.
    Can,
    /// OPC UA.
    OpcUa,
    /// MQTT.
    Mqtt,
    /// Celestia wire protocol (embassy native).
    CelestiaWire,
    /// Unknown / custom protocol (extensible).
    Other(u8),
}

/// An opaque protocol frame — raw bytes tagged with the protocol kind.
///
/// This is a pass-through container. hare does not inspect or modify the
/// bytes; evernight (or the embassy node, for `CelestiaWire`) encodes and
/// decodes them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolFrame {
    /// Which protocol these bytes belong to.
    pub kind: ProtocolKind,
    /// The raw protocol bytes (opaque to hare).
    pub data: Vec<u8>,
}
