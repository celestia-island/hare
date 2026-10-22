<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>A <code>#![no_std]</code> embedded bridge library for the Celestia IoT ecosystem</strong></p>

<div align="center">

[![CI](https://github.com/celestia-island/hare/actions/workflows/ci.yml/badge.svg)](https://github.com/celestia-island/hare/actions/workflows/ci.yml)
[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![crates.io](https://img.shields.io/crates/v/hare)](https://crates.io/crates/hare)
[![docs.rs](https://img.shields.io/docsrs/hare)](https://docs.rs/hare)

</div>

<div align="center">

**English** ·
[简体中文](./docs/zh-Hans/README.md) ·
[繁體中文](./docs/zh-Hant/README.md) ·
[日本語](./docs/ja/README.md) ·
[한국어](./docs/ko/README.md) ·
[Français](./docs/fr/README.md) ·
[Español](./docs/es/README.md) ·
[Русский](./docs/ru/README.md) ·
[العربية](./docs/ar/README.md)

</div>

Hare is the shared contract layer between **embassy-based sensor nodes** (bare-metal
MCUs) and the **gateway broker** — today [`evernight`](https://github.com/celestia-island/evernight)
running on Linux. Both sides deserialize the same hardware manifest and speak the same
wire framing, so a sensor firmware and a gateway service can be built independently and
still meet in the middle.

It is `#![no_std]` + `alloc`: no std, no tokio, no OS calls, and no async runtime of its
own — the embassy node provides the transport, hare provides the protocol.

## Features

- **Manifest schema** (`hare::manifest`) — hardware descriptions: register maps, alarm
  rules, station configs, and scale transforms. The same schema both sides deserialize.
- **Wire protocol** (`hare::wire`) — a compact binary framing protocol for
  UART/USB-CDC/SPI between sensor nodes and the gateway: `0xCE` magic, `u16` length,
  `postcard` payload, CRC16-Modbus trailer.
- **Protocol type identifiers** (`hare::codec`) — `ProtocolKind` and an opaque
  `ProtocolFrame` container for Modbus, MC Protocol, EtherNet/IP-CIP, S7comm, and CAN.
  The encode/decode logic itself belongs to evernight, not to hare.
- **HAL traits** (`hare::hal`) — `Transport` and `SensorDevice` traits that embassy nodes
  implement against their own hardware.
- **ANSI/VT parser** (`hare::ansi`) — a state-machine escape-sequence parser that emits
  semantic events, for terminals and protocol loggers that need to understand a byte
  stream without owning a screen model.

## Installation

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false` is the right choice for an embassy firmware: the defaults are
the full no_std set (`wire`, `manifest`, `codec`, `hal`, `ansi`), and a firmware usually
only needs the modules it links.

### Feature flags

| Feature | Default | What it enables |
|---|---|---|
| `wire` | ✅ | Celestia wire protocol: `Node` / `Gateway` / `Frame`, postcard payloads, CRC16 |
| `manifest` | ✅ | Hardware manifest schema (`HardwareManifest`, register maps, alarms, scales) |
| `codec` | ✅ | Protocol type identifiers and the opaque `ProtocolFrame` container |
| `hal` | ✅ | `Transport` / `SensorDevice` traits — embassy-implementable |
| `ansi` | ✅ | ANSI/VT escape-sequence parser (state machine, no_std) |
| `std` | ❌ | Std-only helpers (TOML/JSON manifest adapters) for the host side |

## Quick start

### Sensor node (embassy firmware)

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

The full reference is [`examples/embassy_node.rs`](./examples/embassy_node.rs); it does
not compile on the host (it needs a board HAL) and is meant to be copied into a firmware
project.

### Gateway side (evernight)

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

A host-only end-to-end demo lives in [`examples/host_demo.rs`](./examples/host_demo.rs).

### End-to-end on an emulated MCU

The workspace member [`qemu-mps2/`](./qemu-mps2/) is a Cortex-M4 firmware demo that runs
under QEMU and reports temperature telemetry over UART; `examples/host_gateway.rs` in the
same directory decodes those frames on the host. It doubles as the wire-protocol test
harness:

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## Design principles

1. **`#![no_std]` + `alloc` only** — no std, no tokio, no OS calls. The wire layer
   allocates frame buffers through `alloc`, so budget for a heap on the target rather than
   a fixed figure.
2. **Serde everywhere** — manifest types derive `Serialize`/`Deserialize`; wire payloads
   use [`postcard`](https://docs.rs/postcard), a compact no_std binary format.
3. **Codec ≠ transport** — encode/decode are pure functions returning `Vec<u8>`; the
   caller owns I/O (embassy async, tokio, kernel IRQ — all work).
4. **Shared schema, independent implementations** — embassy nodes and evernight both
   deserialize the same `HardwareManifest`, but their runtime converters stay separate
   (no_std cannot use closures or `Arc<dyn Fn>`).

## Relationship to the KEI kernel

Hare was extracted from the [`kei`](https://github.com/celestia-island/kei) repository,
where it lived under `packages/kei/`. The kernel is **frozen** ([roadmap](https://docs.celestia.world)),
and the bridge crate now lives here so it can keep evolving while the kernel does not.

The two are independent artifacts that share a contract rather than code:

| | KEI kernel (`celestia-island/kei`) | Hare (this crate) |
|---|---|---|
| Target | ARM64 / RISC-V **gateways**, Linux syscall ABI, MMU required | bare-metal **MCUs**, no MMU |
| Toolchain | nightly, `aarch64-unknown-none` family | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| Role | long-term candidate to replace Linux on the gateway | the MCU-side contract |

This repository is self-contained: stable Rust, its own `rust-toolchain.toml`, and a
workspace whose only member (`qemu-mps2/`) lives inside this directory. Cloning it gives
you a working workspace — no kernel checkout and no `rustup` override needed.

## Targets and CI

| Gate | Target |
|---|---|
| `test` | host (`x86_64`), all features |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, the primary embassy target) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (secondary target) |
| `fmt` / `clippy` / `bench-check` | host, `--all-targets` |

Everything runs on the organization's self-hosted runners; the workflow is
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

## Documentation

- API reference: [docs.rs/hare](https://docs.rs/hare)
- Translations: [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

## License

SySL-1.0 (Synthetic Source License) — see [LICENSE](./LICENSE).

---

## Provenance

Extracted verbatim from the [`kei`](https://github.com/celestia-island/kei) kernel repo
(`packages/kei/`) on 2026-10-20 by user ruling, renamed `kei-wire` at creation and to
`hare` (Omagari Hare, 小鉤晴) on 2026-10-21 by user ruling. The kernel keeps its freeze;
this repo carries the bridge crates (wire / manifest / codec / hal) forward.
