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

Hare 是 **基于 embassy 的传感器节点**（裸机 MCU）与 **网关 broker** 之间的共享契约层——
目前由运行在 Linux 上的 [`evernight`](https://github.com/celestia-island/evernight) 承担后者。
两端反序列化同一份硬件清单、使用同一套线上分帧格式，因此传感器固件与网关服务可以各自
独立构建，最终仍能在中间顺利对接。

它只使用 `#![no_std]` + `alloc`：没有 std、没有 tokio、不发起任何操作系统调用，也不自带
异步运行时——传输由 embassy 节点提供，协议由 hare 提供。

## 功能特性

- **清单 schema**（`hare::manifest`）——硬件描述：寄存器映射、告警规则、站点配置与
  量程变换。两端反序列化的是同一份 schema。
- **Wire 线协议**（`hare::wire`）——传感器节点与网关之间 UART/USB-CDC/SPI 链路上的
  紧凑二进制分帧协议：`0xCE` 魔数、`u16` 长度、`u8` 消息类型、`postcard` 载荷、
  CRC16-Modbus 尾部校验。
- **协议类型标识**（`hare::codec`）——`ProtocolKind`，以及一个不透明的 `ProtocolFrame`
  容器：它让厂商私有帧穿过 wire 层，而无需 hare 去解析。
- **协议中枢**（`hare::protocol`）——家族自有的协议资产：Modbus CRC16 与寄存器/线圈
  提取、三菱 MC Protocol 3E 的请求/响应分帧，以及带 DB → SCL 报告的 S7comm 区域/参数
  模型。需要完整第三方客户端栈的协议仍留在网关服务中。
- **HAL trait**（`hare::hal`）——`Transport` 与 `SensorDevice` 两个 trait，由 embassy
  节点针对自身硬件实现。
- **ANSI/VT 解析器**（`hare::ansi`）——一个输出语义事件的状态机式转义序列解析器，面向
  需要理解字节流、但不想自己维护屏幕模型的终端与协议日志器。

## 安装

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

对 embassy 固件而言，`default-features = false` 是正确选择：默认特性是完整的 no_std 集合
（`wire`、`manifest`、`codec`、`hal`、`ansi`），而固件通常只需要它实际链接的那些模块。

### 特性开关

| 特性 | 默认 | 启用内容 |
|---|---|---|
| `wire` | ✅ | Celestia wire 协议：`Node` / `Gateway` / `Frame`、postcard 载荷、CRC16 |
| `manifest` | ✅ | 硬件清单 schema（`HardwareManifest`、寄存器映射、告警、量程） |
| `codec` | ✅ | 协议类型标识与不透明的 `ProtocolFrame` 容器 |
| `hal` | ✅ | `Transport` / `SensorDevice` trait——可由 embassy 实现 |
| `ansi` | ✅ | ANSI/VT 转义序列解析器（状态机，no_std） |
| `std` | ❌ | 在主机侧构建中把 `std` 转发给可选的 `serde` / `postcard` 依赖 |

`hare::protocol` 始终会被编译——它不引入任何依赖。

清单就是普通的 serde 数据。hare **不**自带 TOML 或 JSON 适配器：请用你选定的 serde 格式
加载它（主机演示使用的是 dev-dependency `serde_json`）。

## 快速开始

### 传感器节点（embassy 固件）

```rust
use hare::manifest::SensorUnit;
use hare::wire::{Node, NodeState, Request};

let mut node = Node::new(transport, 1);      // station 1, over your embassy UART
let mut sensor = MyTempSensor;               // implements hare::hal::SensorDevice
node.send_status(NodeState::Boot, "firmware v0.1", 0).ok();

loop {
    match node.poll(&mut sensor) {           // sync; drive it from an embassy task
        Ok(Request::ReadRegister(req)) => {
            let value = sensor.celsius(req.register);
            node.send_telemetry(req.register, value, SensorUnit::Celsius, 0).ok();
        }
        Ok(Request::Discover(_)) => {
            node.send_discover_response("ntc-temp-sensor", 2).ok();
        }
        Ok(_) => {}
        Err(_) => { /* transport error: re-init the UART or reset */ }
    }
}
```

`Node` 在设计上就是同步的：它在内部循环调用 `Transport::recv`，因此可与任意传输方式配合
（embassy UART、RS-485 收发器、测试管道），并且不引入任何运行时依赖。完整参考实现见
[`examples/embassy_node.rs`](../../examples/embassy_node.rs)——这是一个能在主机上以 stub
形式编译通过的模板，真实的板级接线写在其被注释掉的 embassy 主循环里。

### 网关侧（evernight）

```rust
use hare::wire::{Gateway, Incoming};

let mut gateway = Gateway::new(transport);   // serial port / USB-CDC / socket
gateway.send_discover()?;

loop {
    match gateway.recv()? {
        Incoming::Telemetry(t) => println!(
            "station {} register 0x{:04X} = {} {:?}",
            t.station_id, t.register, t.value, t.unit
        ),
        Incoming::DiscoverResponse(d) => println!(
            "found {} (station {}, {} registers)",
            d.name, d.station_id, d.register_count
        ),
        Incoming::Alarm(a) => eprintln!("alarm at station {}: {}", a.station_id, a.message),
        Incoming::Status(s) => println!("station {} is {:?}", s.station_id, s.state),
        Incoming::Nack(n) => eprintln!("node {} rejected: {}", n.station_id, n.message),
    }
}
```

一份完整的纯主机演练（发现、寄存器读取、遥测）见
[`examples/host_demo.rs`](../../examples/host_demo.rs)。

### 在模拟 MCU 上做端到端验证

工作区成员 [`qemu-mps2/`](../../qemu-mps2/) 是一个 Cortex-M4 固件演示，在 QEMU 下运行，
并通过 UART 上报温度遥测；`qemu-mps2/examples/host_gateway.rs` 则在主机侧解码这些帧。
它同时充当 wire 协议的测试装置：

```bash
# Firmware (thumbv7em, 4 MB of flash and RAM in the MPS2-AN386 memory map)
cargo build --release -p kei-qemu-mps2 --target thumbv7em-none-eabi
# Host-side decoder
cargo build -p kei-qemu-mps2 --example host_gateway --target x86_64-unknown-linux-gnu

qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 4M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-mps2 \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## 设计原则

1. **只使用 `#![no_std]` + `alloc`**——没有 std、没有 tokio、不发起任何操作系统调用。
   wire 层通过 `alloc` 分配帧缓冲区，因此在目标端要为堆留出预算，而不是按固定数字估算。
2. **处处使用 Serde**——清单类型派生 `Serialize`/`Deserialize`；wire 载荷使用
   [`postcard`](https://docs.rs/postcard)，一种紧凑的 no_std 二进制格式。
3. **编码保持纯粹，I/O 归调用方**——`Frame::encode` 返回 `Vec<u8>`，`decode_frame` 返回
   解析后的 `Frame`；传输层（embassy UART、tokio 串口、内核 IRQ）与重试策略都由调用方
   自己掌握。
4. **共享 schema，实现各自独立**——embassy 节点与 evernight 都反序列化同一份
   `HardwareManifest`，但两者的运行时转换器保持分离：no_std 节点无法依赖网关的运行时
   类型或 trait 对象。

## 与 KEI 内核的关系

Hare 抽离自 [`kei`](https://github.com/celestia-island/kei) 仓库，原本位于该仓库的
`packages/kei/` 下。内核处于**冻结**状态（[路线图](https://docs.celestia.world)），
桥接 crate 现在位于本仓库，以便在内核停止演进的同时继续迭代。

两者是彼此独立的产物，共享的是契约而非代码：

| | KEI 内核（`celestia-island/kei`） | Hare（本 crate） |
|---|---|---|
| 目标平台 | ARM64 / RISC-V **网关**，Linux 系统调用 ABI，需要 MMU | 裸机 **MCU**，无 MMU |
| 工具链 | nightly，`aarch64-unknown-none` 系列 | stable，`thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| 定位 | 长期候选方案，用于取代网关上的 Linux | MCU 侧的契约 |

本仓库自包含：stable Rust、自带的 `rust-toolchain.toml`，以及一个唯一成员
（`qemu-mps2/`）就位于本目录内的工作区。克隆下来即可得到一个可用的工作区——无需检出
内核，也无需 `rustup` 工具链覆盖（override）。

## 目标平台与 CI

| 门禁 | 目标 |
|---|---|
| `test` | 主机 `x86_64`，全特性 |
| `no_std-check` | `thumbv7em-none-eabi`（Cortex-M，embassy 的主要目标） |
| `no_std-riscv` | `riscv32imc-unknown-none-elf`（次要目标） |
| `fmt` | 主机，`cargo fmt --all --check` |
| `clippy` | 主机，`--all-features --all-targets -- -D warnings` |
| `bench-check` | 主机，`cargo bench --no-run` |

这些 CI 门禁运行在本组织的自托管 runner 上；发布工作流
（`.github/workflows/publish.yml`）则运行在 GitHub 托管的 runner 上，因为本仓库是公开的。
门禁定义位于 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)。

## 文档

- API 参考：[docs.rs/hare](https://docs.rs/hare)
- 翻译版本：[简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

每个 `docs/<language>/README.md` 都是一份译文；若出现分歧，以英文 README 为最终依据。

## 许可证

SySL-1.0（Synthetic Source License，合成源码许可证）——详见 [LICENSE](../../LICENSE)。

---

## 溯源

2026-10-20 经用户裁决，本 crate 的源码模块从 [`kei`](https://github.com/celestia-island/kei)
内核仓库（`packages/kei/`）中抽离；创建时命名为 `kei-wire`，2026-10-21 又经用户裁决更名为
`hare`（Omagari Hare, 小鉤晴）。内核维持其冻结状态；本仓库则继续推进这些桥接 crate
（wire / manifest / codec / hal / ansi）。在该 crate 迁往独立仓库时，README、包元数据与
文档均做了重写。
