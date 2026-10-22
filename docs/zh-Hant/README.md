<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>為 Celestia IoT 生態系打造的 <code>#![no_std]</code> 嵌入式橋接函式庫</strong></p>

<div align="center">

[![CI](https://github.com/celestia-island/hare/actions/workflows/ci.yml/badge.svg)](https://github.com/celestia-island/hare/actions/workflows/ci.yml)
[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![crates.io](https://img.shields.io/crates/v/hare)](https://crates.io/crates/hare)
[![docs.rs](https://img.shields.io/docsrs/hare)](https://docs.rs/hare)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zh-Hans/README.md) ·
**繁體中文** ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

Hare 是**以 embassy 為基礎的感測節點**（裸機 MCU）與**閘道 broker** 之間的共用契約層 ——
目前閘道端是執行於 Linux 的 [`evernight`](https://github.com/celestia-island/evernight)。
兩端反序列化同一份硬體清單，並使用同一套線路訊框格式，因此感測器韌體與閘道服務可以各自
獨立開發，最後仍能在此契約上會合。

它只使用 `#![no_std]` + `alloc`：沒有 std、沒有 tokio、不做任何 OS 呼叫，本身也不帶非同步
執行期 —— 傳輸由 embassy 節點提供，協定由 hare 提供。

## 功能特色

- **清單結構描述（manifest schema）**（`hare::manifest`）—— 硬體描述：暫存器映射、告警規則、
  站點設定與刻度轉換。這是兩端共同反序列化的同一份結構描述。
- **線路協定（wire protocol）**（`hare::wire`）—— 感測節點與閘道之間 UART/USB-CDC/SPI 使用的
  精簡二進位訊框協定：`0xCE` 魔術數字、`u16` 長度、`postcard` 承載資料、CRC16-Modbus 結尾
  檢查碼。
- **協定型別識別碼**（`hare::codec`）—— `ProtocolKind`，以及供 Modbus、MC Protocol、
  EtherNet/IP-CIP、S7comm 與 CAN 使用的不透明（opaque）`ProtocolFrame` 容器。編解碼邏輯本身屬於
  evernight，不屬於 hare。
- **HAL trait**（`hare::hal`）—— `Transport` 與 `SensorDevice` 兩個 trait，由 embassy 節點針對
  自身硬體實作。
- **ANSI/VT 解析器**（`hare::ansi`）—— 以狀態機實作的跳脫序列（escape sequence）解析器，
  輸出語意事件，適用於需要理解位元組串流、但不想自己維護螢幕模型的終端機與協定紀錄器。

## 安裝

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

對 embassy 韌體而言，`default-features = false` 是正確選擇：預設值代表完整的 no_std 集合
（`wire`、`manifest`、`codec`、`hal`、`ansi`），而韌體通常只需要它實際連結到的模組。

### 功能旗標

| 功能旗標 | 預設 | 啟用的內容 |
|---|---|---|
| `wire` | ✅ | Celestia 線路協定：`Node` / `Gateway` / `Frame`、postcard 承載資料、CRC16 |
| `manifest` | ✅ | 硬體清單結構描述（`HardwareManifest`、暫存器映射、告警、刻度轉換） |
| `codec` | ✅ | 協定型別識別碼，以及不透明的 `ProtocolFrame` 容器 |
| `hal` | ✅ | `Transport` / `SensorDevice` 兩個 trait —— 可由 embassy 實作 |
| `ansi` | ✅ | ANSI/VT 跳脫序列解析器（狀態機、no_std） |
| `std` | ❌ | 僅限 std 的輔助工具（TOML/JSON 硬體清單轉接器），供主機端使用 |

## 快速開始

### 感測節點（embassy 韌體）

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

完整範例請見 [`examples/embassy_node.rs`](./examples/embassy_node.rs)；它無法在主機上編譯
（需要開發板的 HAL），用途是複製到韌體專案中使用。

### 閘道端（evernight）

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

僅供主機端執行的端到端示範位於 [`examples/host_demo.rs`](./examples/host_demo.rs)。

### 在模擬 MCU 上做端到端驗證

工作區成員 [`qemu-mps2/`](./qemu-mps2/) 是一份 Cortex-M4 韌體示範，可在 QEMU 下執行並透過
UART 回報溫度遙測資料；同一目錄下的 `examples/host_gateway.rs` 則在主機端解碼這些訊框。
它同時兼任線路協定的測試載具：

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## 設計原則

1. **僅使用 `#![no_std]` + `alloc`** —— 沒有 std、沒有 tokio、不做任何 OS 呼叫。線路層會
   透過 `alloc` 配置訊框緩衝區，因此請為目標端預留堆積空間，而不是假設一個固定數值。
2. **全面採用 Serde** —— 清單型別衍生 `Serialize`/`Deserialize`；線路承載資料使用
   [`postcard`](https://docs.rs/postcard)，一種精簡的 no_std 二進位格式。
3. **編解碼 ≠ 傳輸** —— 編碼／解碼是回傳 `Vec<u8>` 的純函式；I/O 由呼叫方負責
   （embassy 非同步、tokio、核心 IRQ —— 都能搭配）。
4. **共用結構描述、各自實作** —— embassy 節點與 evernight 都會反序列化同一份
   `HardwareManifest`，但雙方的執行期轉換器各自獨立（no_std 無法使用閉包或 `Arc<dyn Fn>`）。

## 與 KEI 核心的關係

Hare 是從 [`kei`](https://github.com/celestia-island/kei) 倉庫中抽出來的，原本位於
`packages/kei/`。該核心已經**凍結**（[roadmap](https://docs.celestia.world)），橋接 crate
因此移到此處，讓它能在核心停止演進的同時持續發展。

兩者是獨立產物，共用的是契約而不是程式碼：

| | KEI 核心（`celestia-island/kei`） | Hare（本 crate） |
|---|---|---|
| 目標平台 | ARM64 / RISC-V **閘道**、Linux 系統呼叫 ABI、需要 MMU | 裸機 **MCU**、不需要 MMU |
| 工具鏈 | nightly、`aarch64-unknown-none` 系列 | stable、`thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| 定位 | 長期目標是取代閘道上的 Linux | MCU 端的契約 |

本倉庫自成一體：使用 stable Rust、自帶 `rust-toolchain.toml`，工作區唯一的成員
（`qemu-mps2/`）就在此目錄之下。直接 clone 即可得到可用的工作區 —— 不需要另行 checkout
核心，也不需要 `rustup` override。

## 目標平台與 CI

| 檢查關卡 | 目標平台 |
|---|---|
| `test` | 主機（`x86_64`）、啟用全部功能旗標 |
| `no_std-check` | `thumbv7em-none-eabi`（Cortex-M，embassy 的主要目標平台） |
| `no_std-riscv` | `riscv32imc-unknown-none-elf`（次要目標平台） |
| `fmt` / `clippy` / `bench-check` | 主機、`--all-targets` |

所有作業都跑在組織的自架 runner 上；工作流程定義在
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml)。

## 文件

- API 參考文件：[docs.rs/hare](https://docs.rs/hare)
- 翻譯版本：[简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

本文件為翻譯版本：各語言文件皆是英文版 README 的翻譯，若譯文與英文版有任何出入，一律以
英文版 README 為準。

## 授權條款

SySL-1.0（Synthetic Source License，合成原始碼授權）—— 詳見 [LICENSE](./LICENSE)。

---

## 來源沿革

依使用者裁決，於 2026-10-20 自 [`kei`](https://github.com/celestia-island/kei) 核心倉庫
（`packages/kei/`）原樣抽出；建立時名為 `kei-wire`，並於 2026-10-21 經使用者裁決更名為
`hare`（Omagari Hare，小鉤晴）。核心維持凍結狀態；本倉庫則承接橋接 crate
（wire / manifest / codec / hal）繼續前進。
