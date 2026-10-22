//! S7comm 数据模型层——区域码/字段定义/SCL 生成（自 evernight 迁入，
//! 2026-10-21 协议中枢波）。连接与 PDU 引擎（rust7 / snap7-client）留在
//! 消费方传输层；本模块承载**纯数据面**：S7 寻址区域常量、DB 字段模型、
//! 可导入 TIA Portal 的 SCL 源生成。

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// S7 寻址区域（rust7 约定字节值；集中于此使区域码有唯一的家）
// ---------------------------------------------------------------------------

/// S7 内存区域码（PDU 寻址字节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S7Area {
    /// 过程映像输入（I）。
    Inputs,
    /// 过程映像输出（Q）。
    Outputs,
    /// 位存储区（M）。
    Markers,
    /// 数据块（DB，须配合 db_number）。
    DataBase,
    /// 计数器。
    Counters,
    /// 定时器。
    Timers,
    /// 直接外设访问（P）。
    Peripheral,
}

impl S7Area {
    /// rust7 / snap7 线格式区域码。
    #[must_use]
    pub fn code(&self) -> u8 {
        match self {
            Self::Inputs => 0x81,
            Self::Outputs => 0x82,
            Self::Markers => 0x83,
            Self::DataBase => 0x84,
            Self::Counters => 0x1C,
            Self::Timers => 0x1D,
            Self::Peripheral => 0x80,
        }
    }
}

// ---------------------------------------------------------------------------
// SCL 字段模型与 DB 源生成（自 evernight s7comm.rs 逐字迁移 + no_std 化）
// ---------------------------------------------------------------------------

/// SCL 字段定义（DB 生成用）。
#[derive(Debug, Clone)]
pub struct SclFieldDef {
    /// DB 中的字段名。
    pub name: String,
    /// SCL 数据类型。
    pub data_type: SclDataType,
    /// 可选工程单位（如 "°C"、"MPa"）。
    pub unit: Option<String>,
    /// 可选人类可读描述。
    pub description: Option<String>,
}

/// SCL 数据类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SclDataType {
    Bool,
    Byte,
    Word,
    Dword,
    Int,
    Dint,
    Real,
    String,
    Timer,
    Counter,
}

impl SclDataType {
    /// SCL 源中的类型拼写。
    #[must_use]
    pub fn scl_name(&self) -> &'static str {
        match self {
            Self::Bool => "Bool",
            Self::Byte => "Byte",
            Self::Word => "Word",
            Self::Dword => "DWord",
            Self::Int => "Int",
            Self::Dint => "DInt",
            Self::Real => "Real",
            Self::String => "String[254]",
            Self::Timer => "Time",
            Self::Counter => "Int",
        }
    }

    /// BEGIN 段的初始值拼写。
    #[must_use]
    pub fn scl_default(&self) -> &'static str {
        match self {
            Self::Bool => "FALSE",
            Self::Real => "0.0",
            Self::String => "''",
            _ => "0",
        }
    }
}

/// 依据发现到的字段生成完整 SCL DB 源文件（可导入 TIA Portal）。
///
/// 时间戳行由调用方注入（no_std 侧无壁钟）；传 `None` 省略。
#[must_use]
pub fn generate_scl_db_definition(
    db_name: &str,
    fields: &[SclFieldDef],
    generated_at: Option<&str>,
) -> String {
    let mut scl = String::new();
    scl.push_str(&format!(
        "// Auto-generated — DO NOT EDIT MANUALLY\n// Data Block: {db_name}\n"
    ));
    if let Some(ts) = generated_at {
        scl.push_str(&format!("// Generated: {ts}\n"));
    }
    scl.push('\n');
    scl.push_str(&format!(
        "DATA_BLOCK \"{db_name}\"\n{{ S7_Optimized_Access := 'TRUE' }}\nVERSION : 0.1\n   STRUCT\n"
    ));
    for field in fields {
        let comment = if field.unit.is_some() || field.description.is_some() {
            let prefix = field.unit.as_deref().unwrap_or("");
            let desc = field.description.as_deref().unwrap_or("");
            let sep = if field.unit.is_some() && field.description.is_some() {
                " — "
            } else {
                ""
            };
            format!(" // {prefix}{sep}{desc}")
        } else {
            String::new()
        };
        scl.push_str(&format!(
            "   {} : {};{}\n",
            field.name,
            field.data_type.scl_name(),
            comment
        ));
    }
    scl.push_str("   END_STRUCT;\nBEGIN\n");
    for field in fields {
        scl.push_str(&format!(
            "   {} := {};\n",
            field.name,
            field.data_type.scl_default()
        ));
    }
    scl.push_str("END_DATA_BLOCK\n");
    scl
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn area_codes_match_wire_format() {
        assert_eq!(S7Area::Inputs.code(), 0x81);
        assert_eq!(S7Area::Outputs.code(), 0x82);
        assert_eq!(S7Area::Markers.code(), 0x83);
        assert_eq!(S7Area::DataBase.code(), 0x84);
    }

    #[test]
    fn scl_generation_struct_begin_end_round_trip() {
        let fields = vec![
            SclFieldDef {
                name: "Temperature".into(),
                data_type: SclDataType::Real,
                unit: Some("°C".into()),
                description: Some("reactor temp".into()),
            },
            SclFieldDef {
                name: "Running".into(),
                data_type: SclDataType::Bool,
                unit: None,
                description: None,
            },
        ];
        let scl = generate_scl_db_definition("DB100", &fields, Some("2026-10-21 00:00:00 UTC"));
        // 头
        assert!(scl.contains("DATA_BLOCK \"DB100\""));
        assert!(scl.contains("S7_Optimized_Access := 'TRUE'"));
        // 字段（含单位注释）
        assert!(scl.contains("Temperature : Real; // °C — reactor temp"));
        assert!(scl.contains("Running : Bool;"));
        // BEGIN 初值
        assert!(scl.contains("Temperature := 0.0;"));
        assert!(scl.contains("Running := FALSE;"));
        assert!(scl.trim_end().ends_with("END_DATA_BLOCK"));
    }

    #[test]
    fn scl_without_timestamp_omits_line() {
        let scl = generate_scl_db_definition("DB1", &[], None);
        assert!(!scl.contains("Generated:"));
    }
}
