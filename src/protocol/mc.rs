//! 三菱 MC Protocol（3E 二进制帧）编解码——自 evernight 手搓实现迁入
//! （2026-10-21 协议中枢波；唯一非外部库包裹的家族自有协议实现）。
//!
//! 纯构造/解析，无 IO：`build_read_word_request` / `build_write_word_request`
//! 产出的帧可直接写入 TCP 流；`parse_response` 消费完整响应帧。传输
//! （`McProtocolClient` 的连接管理/扫描）留在 evernight。

use alloc::vec::Vec;

/// MC 设备代码（寻址空间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McDevice {
    /// D 软元件（数据寄存器，16 位字）。
    DataRegister,
    /// W 软元件（链接寄存器，16 位字）。
    LinkRegister,
    /// R 软元件（文件寄存器，16 位字）。
    FileRegister,
    /// B 软元件（链接继电器，位）。
    LinkRelay,
    /// X 软元件（输入，位）。
    Input,
    /// Y 软元件（输出，位）。
    Output,
    /// M 软元件（内部继电器，位）。
    InternalRelay,
}

impl McDevice {
    /// ASCII 形式的设备代码字符（3E ASCII 帧与配置面使用）。
    pub fn code_char(&self) -> &'static str {
        match self {
            Self::DataRegister => "D",
            Self::LinkRegister => "W",
            Self::FileRegister => "R",
            Self::LinkRelay => "B",
            Self::Input => "X",
            Self::Output => "Y",
            Self::InternalRelay => "M",
        }
    }

    /// 从设备代码字符解析。
    pub fn from_code_char(s: &str) -> Result<Self, McError> {
        match s {
            "D" => Ok(Self::DataRegister),
            "W" => Ok(Self::LinkRegister),
            "R" => Ok(Self::FileRegister),
            "B" => Ok(Self::LinkRelay),
            "X" => Ok(Self::Input),
            "Y" => Ok(Self::Output),
            "M" => Ok(Self::InternalRelay),
            other => Err(McError::UnknownDevice(other.into())),
        }
    }

    /// 3E 二进制帧的设备代码字节。
    pub fn binary_code(&self) -> u8 {
        match self {
            Self::DataRegister => 0xA8,
            Self::LinkRegister => 0xB4,
            Self::FileRegister => 0xAF,
            Self::LinkRelay => 0xA0,
            Self::Input => 0x9C,
            Self::Output => 0x9D,
            Self::InternalRelay => 0x90,
        }
    }
}

/// MC 编解码错误。
#[derive(Debug)]
pub enum McError {
    UnknownDevice(Vec<u8>),
    /// 响应帧以非成功结束码开头。
    EndCode { code: [u8; 2] },
    /// 帧长不足（协议头/体不完整）。
    MalformedFrame { len: usize },
}

/// 3E 请求副头部（批量读字）。
const CMD_BATCH_READ_WORD: [u8; 2] = [0x01, 0x04];
/// 3E 请求副头部（批量写字）。
const CMD_BATCH_WRITE_WORD: [u8; 2] = [0x01, 0x14];
/// 字软元件子命令。
const SUBCMD_WORD: [u8; 2] = [0x00, 0x00];
/// 3E 请求主头部（0x50 00）。
const SUBHEADER_REQUEST: [u8; 2] = [0x50, 0x00];

/// 构造批量读字请求帧（3E 二进制）。
#[must_use]
pub fn build_read_word_request(
    device: McDevice,
    head_address: u32,
    count: u16,
    network_no: u8,
    pc_no: u8,
    station_no: u8,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(20);
    body.extend_from_slice(&CMD_BATCH_READ_WORD);
    body.extend_from_slice(&SUBCMD_WORD);
    body.extend_from_slice(&head_address.to_le_bytes()[..3]); // 3 字节小端起始地址
    body.push(device.binary_code());
    body.extend_from_slice(&count.to_le_bytes());
    build_3e_frame(&body, network_no, pc_no, station_no)
}

/// 构造批量写字请求帧（3E 二进制）。
#[must_use]
pub fn build_write_word_request(
    device: McDevice,
    head_address: u32,
    values: &[u16],
    network_no: u8,
    pc_no: u8,
    station_no: u8,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(20 + values.len() * 2);
    body.extend_from_slice(&CMD_BATCH_WRITE_WORD);
    body.extend_from_slice(&SUBCMD_WORD);
    body.extend_from_slice(&head_address.to_le_bytes()[..3]);
    body.push(device.binary_code());
    body.extend_from_slice(&(values.len() as u16).to_le_bytes());
    for v in values {
        body.extend_from_slice(&v.to_le_bytes());
    }
    build_3e_frame(&body, network_no, pc_no, station_no)
}

/// 以 3E 帧头包裹命令体。
#[must_use]
fn build_3e_frame(body: &[u8], network_no: u8, pc_no: u8, station_no: u8) -> Vec<u8> {
    let data_len = body.len() as u16;
    let mut frame = Vec::with_capacity(11 + body.len());
    frame.extend_from_slice(&SUBHEADER_REQUEST);
    frame.push(network_no);
    frame.push(pc_no);
    frame.extend_from_slice(&[0xFF, 0x03]); // 目标模块 I/O 号
    frame.push(station_no);
    frame.extend_from_slice(&data_len.to_le_bytes());
    frame.extend_from_slice(&[0x00, 0x00]); // 保留
    frame.extend_from_slice(body);
    frame
}

/// 解析后的 MC 响应。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McResponse {
    /// 结束代码（[0x00, 0x00] = 成功）。
    pub end_code: [u8; 2],
    /// 读到的字数据（写响应为空）。
    pub words: Vec<u16>,
}

impl McResponse {
    pub fn is_success(&self) -> bool {
        self.end_code == [0x00, 0x00]
    }
}

/// 解析完整 MC 响应帧（3E 二进制响应：副头 0xD0 00 + 结束码 + 数据）。
pub fn parse_response(frame: &[u8]) -> Result<McResponse, McError> {
    // 最小响应 = 副头(2) + 结束码(2) = 4 字节
    if frame.len() < 4 {
        return Err(McError::MalformedFrame { len: frame.len() });
    }
    let end_code = [frame[2], frame[3]];
    let data = if frame.len() > 4 { &frame[4..] } else { &[][..] };
    let words = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| u16::from_le_bytes(*chunk))
        .collect();
    Ok(McResponse { end_code, words })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn read_request_golden_bytes() {
        // D100 起 10 字：帧头 11 字节 + 体 10 字节（副头2+子命令2+地址3+设备1+数量2）
        let frame = build_read_word_request(McDevice::DataRegister, 100, 10, 0, 0, 0);
        assert_eq!(frame.len(), 21);
        assert_eq!(&frame[..2], &[0x50, 0x00]); // 3E 请求副头
        assert_eq!(&frame[11..13], &CMD_BATCH_READ_WORD);
        assert_eq!(frame[17], 0xA8); // D 设备码
        assert_eq!(frame[18..20], [10, 0]); // 数量 LE
    }

    #[test]
    fn write_request_carries_values_little_endian() {
        let frame = build_write_word_request(
            McDevice::DataRegister,
            200,
            &[0x1234, 0x5678],
            1,
            2,
            3,
        );
        // 体 = 副头2+子2+地址3+设备1+数量2+数据4 = 14；帧 = 11+14
        assert_eq!(frame.len(), 25);
        assert_eq!(&frame[19..21], &[0x34, 0x12]);
        assert_eq!(&frame[21..23], &[0x78, 0x56]);
        // 寻址字段回读
        assert_eq!(frame[1], 1); // network
        assert_eq!(frame[2], 2); // pc
        assert_eq!(frame[5], 3); // station
    }

    #[test]
    fn response_parse_success_with_words() {
        // 副头 D0 00 + 结束码 00 00 + 两字 LE
        let frame = [0xD0, 0x00, 0x00, 0x00, 0x34, 0x12, 0x78, 0x56];
        let resp = parse_response(&frame).unwrap();
        assert!(resp.is_success());
        assert_eq!(resp.words, vec![0x1234, 0x5678]);
    }

    #[test]
    fn response_parse_error_end_code() {
        let frame = [0xD0, 0x00, 0x40, 0x00]; // 结束码 0x0040
        let resp = parse_response(&frame).unwrap();
        assert!(!resp.is_success());
        assert!(resp.words.is_empty());
    }

    #[test]
    fn malformed_short_frame_rejected() {
        assert!(matches!(
            parse_response(&[0xD0, 0x00, 0x00]),
            Err(McError::MalformedFrame { len: 3 })
        ));
    }

    #[test]
    fn device_codes_round_trip() {
        for d in [
            McDevice::DataRegister,
            McDevice::LinkRegister,
            McDevice::FileRegister,
            McDevice::LinkRelay,
            McDevice::Input,
            McDevice::Output,
            McDevice::InternalRelay,
        ] {
            assert_eq!(McDevice::from_code_char(d.code_char()).unwrap(), d);
        }
        assert!(McDevice::from_code_char("Q").is_err());
    }
}
