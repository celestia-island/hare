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
今日的閘道端是執行於 Linux 的 [`evernight`](https://github.com/celestia-island/evernight)。
兩端會反序列化同一份硬體清單、使用同一套線路訊框格式，因此感測器韌體與閘道服務可以各自
獨立建置，最終仍能在中間會合。

它只使用 `#![no_std]` + `alloc`：沒有 std、沒有 tokio、不做任何 OS 呼叫，本身也不帶非同步
執行期 —— 傳輸由 embassy 節點提供，協定由 hare 提供。

## 功能特色

- **清單結構描述**（`hare::manifest`）—— 硬體描述：暫存器映射、告警規則、站點設定與刻度
  轉換。兩端反序列化的是同一份結構描述。
- **線路協定**（`hare::wire`）—— 感測節點與閘道之間 UART/USB-CDC/SPI 使用的精簡二進位訊框
  協定：`0xCE` 魔術數字、`u16` 長度、`u8` 訊息型別、`postcard` 承載資料、CRC16-Modbus 結尾
  檢查碼。
- **協定型別識別碼**（`hare::codec`）—— `ProtocolKind`，以及供 Modbus、MC Protocol、
  EtherNet/IP-CIP、S7comm 與 CAN 使用的不透明 `ProtocolFrame` 容器。編解碼邏輯本身屬於
  evernight，不屬於 hare。
- **HAL trait**（`hare::hal`）—— `Transport` 與 `SensorDevice` 兩個 trait，由 embassy 節點針對
  自身硬體實作。
- **ANSI/VT 解析器**（`hare::ansi`）—— 以狀態機實作的跳脫序列解析器，會發出語意事件，適用於
  需要理解位元組串流、但不想自己持有螢幕模型的終端機與協定紀錄器。

## 安裝

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

對 embassy 韌體而言，`default-features = false` 是正確選擇：預設值是完整的 no_std 集合
（`wire`、`manifest`、`codec`、`hal`、`ansi`），而韌體通常只需要它實際連結到的模組。

### 功能旗標

| 功能旗標 | 預設 | 啟用的內容 |
|---|---|---|
| `wire` | ✅ | Celestia 線路協定：`Node` / `Gateway` / `Frame`、postcard 承載資料、CRC16 |
| `manifest` | ✅ | 硬體清單結構描述（`HardwareManifest`、暫存器映射、告警、刻度轉換） |
| `codec` | ✅ | 協定型別識別碼，以及不透明的 `ProtocolFrame` 容器 |
| `hal` | ✅ | `Transport` / `SensorDevice` 兩個 trait —— 可由 embassy 實作 |
| `ansi` | ✅ | ANSI/VT 跳脫序列解析器（狀態機、no_std） |
| `std` | ❌ | 在主機端建置時，將 `std` 轉發給選用的 `serde` / `postcard` 相依套件 |

清單本身只是普通的 serde 資料。hare **不**隨附任何 TOML 或 JSON 轉接器：請用你偏好的 serde
格式載入它（主機端示範使用 `serde_json`，屬於 dev-dependency）。

## 快速開始

### 感測節點（embassy 韌體）

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

`Node` 在設計上就是同步的：它本身會在 `Transport::recv` 上迴圈，因此可搭配任何傳輸（embassy
UART、RS-485 收發器、測試用管線），也不會引入任何執行期相依。完整參考實作位於
[`examples/embassy_node.rs`](../../examples/embassy_node.rs) —— 那是一份在主機上以空殼（stub）
形式編譯的範本，真正的開發板接線則寫在其註解掉的 embassy 主迴圈裡。

### 閘道端（evernight）

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

一份僅在主機端執行的完整逐步導覽（探索、暫存器讀取、遙測）位於
[`examples/host_demo.rs`](../../examples/host_demo.rs)。

### 在模擬 MCU 上做端到端驗證

工作區成員 [`qemu-mps2/`](../../qemu-mps2/) 是一份 Cortex-M4 韌體示範，可在 QEMU 下執行並透過
UART 回報溫度遙測資料；`qemu-mps2/examples/host_gateway.rs` 則在主機端解碼這些訊框。它同時
兼任線路協定的測試載具：

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

## 設計原則

1. **僅使用 `#![no_std]` + `alloc`** —— 沒有 std、沒有 tokio、不做任何 OS 呼叫。線路層會透過
   `alloc` 配置訊框緩衝區，因此請為目標端的堆積預留空間，而不是假設一個固定數值。
2. **全面採用 Serde** —— 清單型別衍生 `Serialize`/`Deserialize`；線路承載資料使用
   [`postcard`](https://docs.rs/postcard)，一種精簡的 no_std 二進位格式。
3. **編碼是純函式，I/O 屬於呼叫方** —— `Frame::encode` 回傳 `Vec<u8>`，`decode_frame` 回傳
   解析後的 `Frame`；傳輸（embassy UART、tokio serial、核心 IRQ）與重試策略由呼叫方負責。
4. **共用結構描述、各自實作** —— embassy 節點與 evernight 都會反序列化同一份
   `HardwareManifest`，但雙方的執行期轉換器各自獨立：no_std 節點無法相依於閘道的執行期型別
   或 trait 物件。

## 與 KEI 核心的關係

Hare 是從 [`kei`](https://github.com/celestia-island/kei) 倉庫中抽出來的，原本位於
`packages/kei/`。該核心已經**凍結**（[roadmap](https://docs.celestia.world)），橋接 crate 現在
搬到此處，讓它能在核心停止演進的同時持續發展。

兩者是獨立產物，共用的是契約而不是程式碼：

| | KEI 核心（`celestia-island/kei`） | Hare（本 crate） |
|---|---|---|
| 目標平台 | ARM64 / RISC-V **閘道**、Linux 系統呼叫 ABI、需要 MMU | 裸機 **MCU**、不需要 MMU |
| 工具鏈 | nightly、`aarch64-unknown-none` 系列 | stable、`thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| 定位 | 長期而言是取代閘道上 Linux 的候選方案 | MCU 端的契約 |

本倉庫自成一體：使用 stable Rust、自帶 `rust-toolchain.toml`，而工作區唯一的成員
（`qemu-mps2/`）就位於此目錄之下。直接 clone 即可得到可用的工作區 —— 不需要另行 checkout
核心，也不需要 `rustup` override。

## 目標平台與 CI

| 檢查關卡 | 目標平台 |
|---|---|
| `test` | 主機 `x86_64`、啟用全部功能旗標 |
| `no_std-check` | `thumbv7em-none-eabi`（Cortex-M，embassy 的主要目標平台） |
| `no_std-riscv` | `riscv32imc-unknown-none-elf`（次要目標平台） |
| `fmt` | 主機、`cargo fmt --all --check` |
| `clippy` | 主機、`--all-features --all-targets -- -D warnings` |
| `bench-check` | 主機、`cargo bench --no-run` |

CI 關卡在組織的自架 runner 上執行；由於本倉庫是公開的，發布工作流程
（`.github/workflows/publish.yml`）則跑在 GitHub 託管的 runner 上。關卡定義位於
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)。

## 文件

- API 參考文件：[docs.rs/hare](https://docs.rs/hare)
- 翻譯版本：[简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

每個 `docs/<language>/README.md` 都是一份翻譯；若譯文與英文版有所出入，一律以英文版 README
為準。

## 授權條款

SySL-1.0（Synthetic Source License）—— 詳見 [LICENSE](../../LICENSE)。

---

## 來源沿革

本 crate 的原始碼模組是在 2026-10-20 依使用者裁決，從
[`kei`](https://github.com/celestia-island/kei) 核心倉庫（`packages/kei/`）抽出的；建立時名為
`kei-wire`，並在 2026-10-21 依使用者裁決更名為 `hare`（Omagari Hare, 小鉤晴）。核心維持凍結
狀態；本倉庫則承接橋接 crate（wire / manifest / codec / hal / ansi）繼續前進。README、套件
中介資料與文件都在 crate 遷移至自有倉庫時重新撰寫。
