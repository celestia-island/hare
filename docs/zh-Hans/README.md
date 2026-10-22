<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>面向 Celestia IoT 生态的 <code>#![no_std]</code> 嵌入式桥接库</strong></p>

<div align="center">

[![CI](https://github.com/celestia-island/hare/actions/workflows/ci.yml/badge.svg)](https://github.com/celestia-island/hare/actions/workflows/ci.yml)
[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![crates.io](https://img.shields.io/crates/v/hare)](https://crates.io/crates/hare)
[![docs.rs](https://img.shields.io/docsrs/hare)](https://docs.rs/hare)

</div>

<div align="center">

[English](../en/README.md) ·
**简体中文** ·
[繁體中文](../zh-Hant/README.md) ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

Hare 是**基于 embassy 的传感器节点**（裸机 MCU）与**网关代理程序**（gateway broker）之间的共享契约层——
后者的当前实现是运行在 Linux 上的 [`evernight`](https://github.com/celestia-island/evernight)。
两端反序列化同一份硬件清单、使用同一套线上分帧格式，因此传感器固件与网关服务可以各自
独立构建，最终仍能在中间顺利对接。

它只使用 `#![no_std]` + `alloc`：不依赖 std、不依赖 tokio，也不发起任何操作系统调用，且
不自带异步运行时——传输由 embassy 节点提供，协议由 hare 提供。

## 功能特性

- **清单 schema**（`hare::manifest`）——硬件描述：寄存器映射、告警规则、站点配置与
  量程变换。两端反序列化的是同一份 schema。
- **Wire 协议**（`hare::wire`）——面向传感器节点与网关之间 UART/USB-CDC/SPI 链路的
  紧凑二进制分帧协议：`0xCE` 魔数、`u16` 长度、`postcard` 载荷、CRC16-Modbus 尾部校验。
- **协议类型标识**（`hare::codec`）——`ProtocolKind`，以及用于 Modbus、MC Protocol、
  EtherNet/IP-CIP、S7comm 和 CAN 的不透明容器 `ProtocolFrame`。编解码逻辑本身属于
  evernight，而不属于 hare。
- **HAL trait 抽象**（`hare::hal`）——`Transport` 与 `SensorDevice` 两个 trait，由 embassy
  节点针对自身硬件实现。
- **ANSI/VT 解析器**（`hare::ansi`）——一个输出语义事件的状态机式转义序列解析器，面向
  需要理解字节流、但不想自己维护屏幕模型的终端与协议日志器。

## 安装

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

对 embassy 固件而言，`default-features = false` 是正确选择：默认特性是完整的 no_std 集合
（`wire`、`manifest`、`codec`、`hal`、`ansi`），而固件通常只需要它实际链接的那几个模块。

### 特性开关

| 特性 | 默认 | 启用内容 |
|---|---|---|
| `wire` | ✅ | Celestia wire 协议：`Node` / `Gateway` / `Frame`、postcard 载荷、CRC16 |
| `manifest` | ✅ | 硬件清单 schema（`HardwareManifest`、寄存器映射、告警、量程） |
| `codec` | ✅ | 协议类型标识与不透明的 `ProtocolFrame` 容器 |
| `hal` | ✅ | `Transport` / `SensorDevice` trait——可由 embassy 实现 |
| `ansi` | ✅ | ANSI/VT 转义序列解析器（状态机，no_std） |
| `std` | ❌ | 仅依赖 std 的辅助项（TOML/JSON 清单适配器），供主机侧使用 |

## 快速上手

### 传感器节点（embassy 固件）

```rust
use hare::wire::{Node, Frame, MsgType};
use hare::manifest::SensorUnit;

let mut node = Node::new(transport);  // your embassy UART
node.register_station(&station_manifest);

loop {
    let req = node.poll().await;
    let temp = read_adc().await;      // your hardware
    node.telemetry(STATION_ID, REGISTER_TEMP, temp, SensorUnit::Celsius)
        .await;
}
```

完整示例见 [`examples/embassy_node.rs`](./examples/embassy_node.rs)；它无法在主机上编译
（需要开发板的 HAL），用途是复制进固件工程。

### 网关侧（evernight）

```rust
use hare::wire::{Gateway, Frame};
use hare::manifest::HardwareManifest;

let manifest = HardwareManifest::from_toml_str(&toml_str)?;
let mut gw = Gateway::new(transport);      // serial port

let frame = gw.recv().await?;              // Frame from a sensor node
let telemetry = frame.as_telemetry()?;
println!("station {} register {} = {} {}",
    telemetry.station_id, telemetry.register, telemetry.value, telemetry.unit);
```

仅运行在主机上的端到端演示见 [`examples/host_demo.rs`](./examples/host_demo.rs)。

### 在模拟 MCU 上做端到端验证

工作区成员 [`qemu-mps2/`](./qemu-mps2/) 是一个 Cortex-M4 固件演示，在 QEMU 下运行，并通过
UART 上报温度遥测；同目录下的 `examples/host_gateway.rs` 则在主机侧解码这些帧。它同时充当
wire 协议的测试装置：

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## 设计原则

1. **只用 `#![no_std]` + `alloc`**——不依赖 std、不依赖 tokio，也不发起任何操作系统调用。wire 层通过
   `alloc` 申请帧缓冲区，因此目标端要按实际需要预留堆空间，而不是钉死一个固定数值。
2. **全链路使用 Serde**——清单类型派生 `Serialize`/`Deserialize`；wire 载荷使用
   [`postcard`](https://docs.rs/postcard)，一种紧凑的 no_std 二进制格式。
3. **编解码 ≠ 传输**——编解码是返回 `Vec<u8>` 的纯函数，I/O 由调用方负责（embassy 异步、
   tokio、内核 IRQ 都可以）。
4. **共享 schema，实现各自独立**——embassy 节点与 evernight 都反序列化同一份
   `HardwareManifest`，但两者的运行时转换器保持分离（no_std 下无法使用闭包或 `Arc<dyn Fn>`）。

## 与 KEI 内核的关系

Hare 抽离自 [`kei`](https://github.com/celestia-island/kei) 仓库，原本位于该仓库的
`packages/kei/` 下。内核现在处于**冻结**状态（[路线图](https://docs.celestia.world)），
桥接 crate 因此独立到本仓库，以便在内核停止演进的同时继续迭代。

两者是彼此独立的产物，共享的是契约而非代码：

| | KEI 内核（`celestia-island/kei`） | Hare（本 crate） |
|---|---|---|
| 目标平台 | ARM64 / RISC-V **网关**，Linux 系统调用 ABI，需要 MMU | 裸机 **MCU**，无 MMU |
| 工具链 | nightly，`aarch64-unknown-none` 系列 | stable，`thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| 定位 | 长期有望取代网关上的 Linux | MCU 侧的契约 |

本仓库自包含：使用 stable Rust，自带 `rust-toolchain.toml`，工作区只有一个成员
（`qemu-mps2/`），且就在本目录内。克隆下来就是一个可用的工作区——无需检出内核，也无需
配置 `rustup` 的工具链覆盖（override）。

## 目标平台与 CI

| 门禁 | 目标 |
|---|---|
| `test` | 主机（`x86_64`），全特性 |
| `no_std-check` | `thumbv7em-none-eabi`（Cortex-M，embassy 的主要目标） |
| `no_std-riscv` | `riscv32imc-unknown-none-elf`（次要目标） |
| `fmt` / `clippy` / `bench-check` | 主机，`--all-targets` |

全部任务都在组织的自托管 runner 上运行；工作流见
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml)。

## 文档

- API 参考：[docs.rs/hare](https://docs.rs/hare)
- 翻译版本（本页为译文，如有出入以仓库根目录的英文 README 为准）：
  [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

## 许可证

SySL-1.0（Synthetic Source License，合成源码许可证）——详见 [LICENSE](./LICENSE)。

---

## 溯源

2026-10-20 经用户裁决，从 [`kei`](https://github.com/celestia-island/kei) 内核仓库
（`packages/kei/`）逐字抽离；创建时命名为 `kei-wire`，2026-10-21 经用户裁决改为
`hare`（Omagari Hare，小鉤晴）。内核维持其冻结状态；本仓库继续推进桥接 crate
（wire / manifest / codec / hal）。
