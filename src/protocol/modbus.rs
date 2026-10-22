//! Modbus RTU 增值层——家族统一的纯编解码工具（无传输、无 rmodbus 依赖）。
//!
//! 迁入来源（2026-10-21 协议中枢波）：aoba 曾在四处各自实现 `crc16_modbus`
//! （api/core、protocol/runtime、protocol/modbus/mod、status/util），帧长
//! 推断与寄存器值提取也散落在 frame/mod 各 pull 模块。本模块是唯一源；
//! 请求**构造**仍由消费方经 `rmodbus` 完成（其本身 no_std，无需包裹）。
//!
//! 约定：帧 = `[slave_id, function, ...payload, crc_lo, crc_hi]`（CRC 小端
//! 存放，低字节在前）——与 Modbus RTU 规范及 aoba 既有实现一致。

use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// CRC16 —— 全家族唯一实现（poly 0xA001 / init 0xFFFF）
// ---------------------------------------------------------------------------

/// 标准 Modbus RTU CRC16（多项式 0xA001，初值 0xFFFF）。
/// 返回值按规范以低字节在前存放于帧尾。
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= u16::from(b);
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc >>= 1;
                crc ^= 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

/// 向帧尾追加 CRC（就地，小端）。
pub fn append_crc(frame: &mut Vec<u8>) {
    let crc = crc16(frame);
    frame.push((crc & 0xFF) as u8);
    frame.push((crc >> 8) as u8);
}

/// 校验整帧（含尾 2 字节 CRC）尾部校验和；不匹配时报错（含双方值）。
pub fn verify_crc(frame: &[u8]) -> Result<(), ModbusError> {
    if frame.len() < 4 {
        return Err(ModbusError::TooShort { len: frame.len() });
    }
    let data_len = frame.len() - 2;
    let calc = crc16(&frame[..data_len]);
    let wire = u16::from(frame[data_len]) | (u16::from(frame[data_len + 1]) << 8);
    if calc != wire {
        return Err(ModbusError::CrcMismatch { calc, wire });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 错误与异常码
// ---------------------------------------------------------------------------

/// 纯编解码层错误（传输错误由消费方定义）。
#[derive(Debug)]
pub enum ModbusError {
    TooShort {
        len: usize,
    },
    CrcMismatch {
        calc: u16,
        wire: u16,
    },
    /// 从站异常响应（功能码 MSB 置位 + 异常码）。
    Exception {
        function: u8,
        code: ExceptionCode,
    },
    /// 无法推断帧长（字节不足以判定边界）。
    LengthUndecidable {
        len: usize,
    },
}

/// Modbus 标准异常码（从站拒绝原因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionCode {
    IllegalFunction,
    IllegalDataAddress,
    IllegalDataValue,
    SlaveDeviceFailure,
    Acknowledge,
    SlaveDeviceBusy,
    MemoryParityError,
    GatewayPathUnavailable,
    GatewayTargetFailed,
    Unknown(u8),
}

impl ExceptionCode {
    pub fn from_code(code: u8) -> Self {
        match code {
            0x01 => Self::IllegalFunction,
            0x02 => Self::IllegalDataAddress,
            0x03 => Self::IllegalDataValue,
            0x04 => Self::SlaveDeviceFailure,
            0x05 => Self::Acknowledge,
            0x06 => Self::SlaveDeviceBusy,
            0x08 => Self::MemoryParityError,
            0x0A => Self::GatewayPathUnavailable,
            0x0B => Self::GatewayTargetFailed,
            other => Self::Unknown(other),
        }
    }
}

/// 若为异常响应帧（功能码 MSB 置位）则解码异常码。
pub fn parse_exception(frame: &[u8]) -> Result<Option<(u8, ExceptionCode)>, ModbusError> {
    if frame.len() < 3 {
        return Err(ModbusError::TooShort { len: frame.len() });
    }
    let function = frame[1];
    if function & 0x80 == 0 {
        return Ok(None);
    }
    Ok(Some((function & 0x7F, ExceptionCode::from_code(frame[2]))))
}

// ---------------------------------------------------------------------------
// 响应值提取（大端 16 位块 / 位打包）——统一各 pull_* 的手工切片
// ---------------------------------------------------------------------------

/// 从读保持/输入寄存器（FC03/FC04）响应中提取 `u16` 值列表。
/// `frame` 为完整帧（含 CRC）；数据区为 `[id, fc, byte_count, data..., crc_lo, crc_hi]`。
pub fn parse_register_values(frame: &[u8]) -> Result<Vec<u16>, ModbusError> {
    if let Some((_, code)) = parse_exception(frame)? {
        return Err(ModbusError::Exception {
            function: 0x03,
            code,
        });
    }
    if frame.len() < 5 {
        return Err(ModbusError::TooShort { len: frame.len() });
    }
    Ok(frame[3..frame.len() - 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| u16::from_be_bytes(*chunk))
        .collect())
}

/// 从读线圈/离散输入（FC01/FC02）响应中提取位值列表。
/// 位按 Modbus 规范从每字节最低位起 packed。
pub fn parse_bit_values(frame: &[u8], count: usize) -> Result<Vec<bool>, ModbusError> {
    if frame.len() < 5 {
        return Err(ModbusError::TooShort { len: frame.len() });
    }
    let data = &frame[3..frame.len() - 2];
    let mut bits = Vec::with_capacity(count);
    for i in 0..count {
        let byte = data.get(i / 8).copied().unwrap_or(0);
        bits.push(byte & (1 << (i % 8)) != 0);
    }
    Ok(bits)
}

// ---------------------------------------------------------------------------
// 帧长推断 —— aoba frame.rs determine_length 的无传输版
// ---------------------------------------------------------------------------

/// **slave 侧**由已收字节推断完整 RTU 请求帧长；字节不足返回 `None`。
/// 请求形状：异常回显 5 / 定长读单写 8（FC01-06）/ 多写按第 7 字节 byte_count。
/// 收满后仍须 [`verify_crc`]。
pub fn request_length_hint(collected: &[u8]) -> Option<usize> {
    if collected.len() < 2 {
        return None;
    }
    let function = collected[1];
    if function & 0x80 != 0 {
        return Some(5);
    }
    match function {
        0x01..=0x06 => Some(8),
        0x0F | 0x10 => {
            let byte_count = collected.get(6).copied()?;
            Some(9 + byte_count as usize)
        }
        _ => None,
    }
}

/// **master 侧**由已收字节推断完整 RTU 响应帧长；字节不足返回 `None`。
/// 响应形状：异常 5 / 读响应按第 3 字节 byte_count（5+bc）/ 单写回显 8。
/// 注意与请求侧的歧义：FC01-06 的请求定长 8，响应按 bc——两侧须各用各的
/// 推断函数（aoba 旧实现用 rmodbus 的 guess 先试响应形，失败再退定长，
/// 本模块改为显式双函数，消除隐式猜测）。
pub fn response_length_hint(collected: &[u8]) -> Option<usize> {
    if collected.len() < 2 {
        return None;
    }
    let function = collected[1];
    if function & 0x80 != 0 {
        return Some(5);
    }
    match function {
        0x01..=0x04 => {
            // 读响应：id + fc + bc + data + crc
            let byte_count = collected.get(2).copied()?;
            Some(5 + byte_count as usize)
        }
        0x05 | 0x06 | 0x0F | 0x10 => Some(8), // 单写/多写响应均为定长回显
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 设备怪癖 —— 重复载荷修正（aoba 实测：某些网关/串口重复帧体 2-3 次）
// ---------------------------------------------------------------------------

/// 检测并裁剪重复载荷（读函数 0x01..=0x04），重算 byte_count 与 CRC。
/// 返回裁剪后的新帧；无重复时返回原帧内容的克隆。
pub fn trim_duplicate_payload(function: u8, frame: &[u8]) -> Result<Vec<u8>, ModbusError> {
    if !(0x01..=0x04).contains(&function) {
        return Err(ModbusError::Exception {
            function,
            code: ExceptionCode::IllegalFunction,
        });
    }
    if frame.len() < 6 {
        // 6 字节 = id + fc + bc + 1 数据字节 + crc2（单寄存器读的最小完整帧）
        return Err(ModbusError::TooShort { len: frame.len() });
    }
    let (byte_count_index, data_start) = (2usize, 3usize);
    let reported_bc = frame[byte_count_index] as usize;
    let data_total = frame.len() - data_start - 2;
    if data_total == reported_bc {
        return Ok(frame.to_vec());
    }
    if reported_bc == 0 || data_total % reported_bc != 0 {
        return Err(ModbusError::LengthUndecidable { len: frame.len() });
    }
    let mult = data_total / reported_bc;
    if !(2..=3).contains(&mult) {
        return Err(ModbusError::LengthUndecidable { len: frame.len() });
    }
    let first = &frame[data_start..data_start + reported_bc];
    for i in 1..mult {
        if &frame[data_start + i * reported_bc..data_start + (i + 1) * reported_bc] != first {
            return Err(ModbusError::LengthUndecidable { len: frame.len() });
        }
    }
    let mut trimmed = frame[..data_start + reported_bc].to_vec();
    trimmed.truncate(data_start + reported_bc);
    let crc = crc16(&trimmed);
    trimmed.push((crc & 0xFF) as u8);
    trimmed.push((crc >> 8) as u8);
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// 黄金向量：CRC16("123456789") = 0x4B37（Modbus 多项式标准检验值）。
    #[test]
    fn crc16_golden_vector() {
        assert_eq!(crc16(b"123456789"), 0x4B37);
    }

    #[test]
    fn append_then_verify_round_trips() {
        let mut frame = alloc::vec![0x01, 0x03, 0x00, 0x00, 0x00, 0x02];
        append_crc(&mut frame);
        assert!(verify_crc(&frame).is_ok());
        // 翻转一位数据位 → CRC 必炸
        let mut corrupt = frame.clone();
        corrupt[3] ^= 0x01;
        assert!(matches!(
            verify_crc(&corrupt),
            Err(ModbusError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn register_values_parse_big_endian() {
        // id=1 fc=3 bc=4 数据 0x0064 0x00C8 + 正确 CRC
        let mut frame = alloc::vec![0x01, 0x03, 0x04, 0x00, 0x64, 0x00, 0xC8];
        append_crc(&mut frame);
        assert_eq!(
            parse_register_values(&frame).unwrap(),
            alloc::vec![100, 200]
        );
    }

    #[test]
    fn exception_frames_decode() {
        // id=1 fc=0x83(异常03) code=0x02(非法地址)
        let mut frame = alloc::vec![0x01, 0x83, 0x02];
        append_crc(&mut frame);
        let (function, code) = parse_exception(&frame).unwrap().unwrap();
        assert_eq!(function, 0x03);
        assert_eq!(code, ExceptionCode::IllegalDataAddress);
        assert!(matches!(
            parse_register_values(&frame),
            Err(ModbusError::Exception { .. })
        ));
    }

    #[test]
    fn bit_values_unpack_lsb_first() {
        // bc=1 数据 0b00000101 → 前 5 位 [true,false,true,false,false]
        let mut frame = alloc::vec![0x01, 0x02, 0x01, 0x05];
        append_crc(&mut frame);
        let bits = parse_bit_values(&frame, 5).unwrap();
        assert_eq!(bits, alloc::vec![true, false, true, false, false]);
    }

    #[test]
    fn length_hints_split_request_and_response_sides() {
        // —— slave 侧（读请求）——
        assert_eq!(request_length_hint(&[0x01]), None);
        assert_eq!(request_length_hint(&[0x01, 0x83]), Some(5));
        assert_eq!(request_length_hint(&[0x01, 0x06]), Some(8));
        assert_eq!(
            request_length_hint(&[0x01, 0x10, 0x00, 0x00, 0x00, 0x02]),
            None
        );
        assert_eq!(
            request_length_hint(&[0x01, 0x10, 0x00, 0x00, 0x00, 0x02, 0x04]),
            Some(13)
        );
        // —— master 侧（读响应）——
        assert_eq!(response_length_hint(&[0x01, 0x83]), Some(5));
        assert_eq!(response_length_hint(&[0x01, 0x03, 0x04]), Some(9));
        assert_eq!(response_length_hint(&[0x01, 0x06]), Some(8)); // 单写回显
        assert_eq!(response_length_hint(&[0x01, 0x10]), Some(8)); // 多写回显
        assert_eq!(response_length_hint(&[0x01, 0x03]), None); // 需 bc 字节
    }

    #[test]
    fn duplicate_payload_trims_and_recomputes_crc() {
        // 构造 bc=4 的正常帧，然后手工重复数据体 2 份 + 坏 CRC
        let mut original = alloc::vec![0x01, 0x03, 0x04, 0x00, 0x64, 0x00, 0xC8];
        append_crc(&mut original);
        let mut doubled =
            alloc::vec![0x01, 0x03, 0x04, 0x00, 0x64, 0x00, 0xC8, 0x00, 0x64, 0x00, 0xC8];
        append_crc(&mut doubled);
        let trimmed = trim_duplicate_payload(0x03, &doubled).unwrap();
        assert_eq!(trimmed.len(), original.len());
        assert_eq!(trimmed, original);
        assert!(verify_crc(&trimmed).is_ok());
    }
}
