//! 协议中枢——家族共有的工业协议编解码层（2026-10-21 用户裁决扩建）。
//!
//! 定位：**纯编解码 + 数据模型**（`no_std + alloc`，零额外依赖），与传输层
//! （tokio / serialport / socket，留在各消费方）严格分离。外部编解码引擎
//! （rmodbus / rust7 / opcua / rumqttc）仍由各消费方传输层直接使用；本模块
//! 承载的是家族**自有**的协议资产——此前散落在 aoba（Modbus 增值层 ×4 份
//! CRC）与 evernight（MC Protocol 手搓全套、S7 数据模型/SCL 工具）。
//!
//! | 模块 | 内容 | 迁入来源 |
//! |---|---|---|
//! | [`modbus`] | CRC16 唯一源、寄存器/线圈值提取、异常码解码、帧长推断、重复载荷怪癖修正 | aoba core/frame/mod |
//! | [`mc`] | 三菱 MC Protocol（3E 二进制帧）读/写请求构造与响应解析 | evernight mc_protocol |
//! | [`s7comm`] | S7 区域码/参数模型 + DB 结构报告与 SCL 定义生成 | evernight s7comm_blocks |

pub mod mc;
pub mod modbus;
pub mod s7comm;
