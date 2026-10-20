//! Sensor core types — physical units, alarm levels, register modes, raw values.
//!
//! These are the fundamental value types shared between sensor nodes and the
//! gateway. All are plain `serde` types with no OS dependency.

use serde::{Deserialize, Serialize};

/// Physical unit associated with a sensor value.
///
/// Non-exhaustive — new units may be added without a major version bump.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SensorUnit {
    MPa,
    Bar,
    Celsius,
    Ppm,
    Percent,
    PercentLEL,
    Kg,
    Grams,
    Volts,
    Amps,
    Watts,
    Kw,
    Nm3PerHour,
    LitersPerMin,
    MLitersPerMin,
    MicroSiemensPerCm,
    Hours,
    Minutes,
    /// Unitless / dimensionless value.
    #[default]
    Dimensionless,
}

impl SensorUnit {
    /// Returns the canonical short string for this unit (e.g. "°C", "V").
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MPa => "MPa",
            Self::Bar => "bar",
            Self::Celsius => "°C",
            Self::Ppm => "ppm",
            Self::Percent => "%",
            Self::PercentLEL => "%LEL",
            Self::Kg => "kg",
            Self::Grams => "g",
            Self::Volts => "V",
            Self::Amps => "A",
            Self::Watts => "W",
            Self::Kw => "kW",
            Self::Nm3PerHour => "Nm³/h",
            Self::LitersPerMin => "L/min",
            Self::MLitersPerMin => "mL/min",
            Self::MicroSiemensPerCm => "µS/cm",
            Self::Hours => "hours",
            Self::Minutes => "min",
            Self::Dimensionless => "",
        }
    }

    /// Parse a canonical unit string (the inverse of [`Self::as_str`]).
    ///
    /// External consumers of the wire protocol (e.g. the evernight gateway)
    /// cannot construct a [`SensorUnit`] variant directly because the enum is
    /// `#[non_exhaustive]`; this is the sanctioned construction path.
    ///
    /// Returns `Option`, not `Result`: an unrecognised string is a normal lookup
    /// miss (the caller substitutes [`Self::Dimensionless`]), not a parse error —
    /// which is why this is deliberately not an implementation of
    /// `core::str::FromStr`. The name is kept because it is the published
    /// counterpart of [`Self::as_str`] and is called by the gateway.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "MPa" => Some(Self::MPa),
            "bar" => Some(Self::Bar),
            "°C" => Some(Self::Celsius),
            "ppm" => Some(Self::Ppm),
            "%" => Some(Self::Percent),
            "%LEL" => Some(Self::PercentLEL),
            "kg" => Some(Self::Kg),
            "g" => Some(Self::Grams),
            "V" => Some(Self::Volts),
            "A" => Some(Self::Amps),
            "W" => Some(Self::Watts),
            "kW" => Some(Self::Kw),
            "Nm³/h" => Some(Self::Nm3PerHour),
            "L/min" => Some(Self::LitersPerMin),
            "mL/min" => Some(Self::MLitersPerMin),
            "µS/cm" => Some(Self::MicroSiemensPerCm),
            "hours" => Some(Self::Hours),
            "min" => Some(Self::Minutes),
            "" => Some(Self::Dimensionless),
            _ => None,
        }
    }
}

/// Alarm severity level (ISA-18.2 inspired).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AlarmLevel {
    /// Informational (no action needed).
    Info,
    /// Low priority (e.g. approaching a threshold).
    Low,
    /// High priority (action required).
    High,
    /// Critical (immediate action required).
    Critical,
}

/// Register read/write mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RegisterMode {
    /// Read-only (input register / discrete input).
    #[default]
    ReadOnly,
    /// Read-write (holding register / coil).
    ReadWrite,
    /// Write-only (output coil).
    WriteOnly,
}

/// Quality flag for a sensor reading.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Quality {
    /// Reading is fresh and valid.
    Good,
    /// Reading has not been updated within the expected interval.
    Stale,
    /// Read or conversion error occurred.
    Error,
}

/// Raw register value — covers both u16 holding/input registers and coil booleans.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RawValue {
    /// Unsigned 16-bit integer (holding/input register).
    U16(u16),
    /// Signed 16-bit integer (interpreted from raw u16).
    I16(i16),
    /// 32-bit float (reconstructed from two consecutive u16 registers).
    F32(f32),
    /// Boolean (coil / discrete input).
    Bool(bool),
}

impl RawValue {
    /// Convert to an f64 for scale transforms and alarm evaluation.
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::U16(v) => *v as f64,
            Self::I16(v) => *v as f64,
            Self::F32(v) => *v as f64,
            Self::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensor_unit_from_str_round_trips() {
        for unit in [
            SensorUnit::MPa,
            SensorUnit::Bar,
            SensorUnit::Celsius,
            SensorUnit::Ppm,
            SensorUnit::Percent,
            SensorUnit::PercentLEL,
            SensorUnit::Kg,
            SensorUnit::Grams,
            SensorUnit::Volts,
            SensorUnit::Amps,
            SensorUnit::Watts,
            SensorUnit::Kw,
            SensorUnit::Nm3PerHour,
            SensorUnit::LitersPerMin,
            SensorUnit::MLitersPerMin,
            SensorUnit::MicroSiemensPerCm,
            SensorUnit::Hours,
            SensorUnit::Minutes,
            SensorUnit::Dimensionless,
        ] {
            let s = unit.as_str();
            assert_eq!(SensorUnit::from_str(s), Some(unit), "round-trip {s:?}");
        }
    }

    #[test]
    fn sensor_unit_from_str_rejects_unknown() {
        assert_eq!(SensorUnit::from_str("KPa"), None);
        assert_eq!(SensorUnit::from_str("℃"), None);
    }
}
