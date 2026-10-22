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
  `u8` message type, `postcard` payload, CRC16-Modbus trailer.
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
| `std` | ❌ | Forwards `std` to the optional `serde` / `postcard` dependencies for host builds |

The manifest is plain serde data. hare ships **no** TOML or JSON adapters: load it with
the serde format of your choice (the host demo uses `serde_json`, a dev-dependency).

## Quick start

### Sensor node (embassy firmware)

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

`Node` is synchronous by design: it loops on `Transport::recv` itself, so it works with
any transport (embassy UART, RS-485 transceiver, a test pipe) and adds no runtime
dependency. The full reference lives in
[`examples/embassy_node.rs`](./examples/embassy_node.rs) — a template that compiles on the
host as a stub and carries the real board wiring in its commented embassy main loop.

### Gateway side (evernight)

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

A complete host-only walkthrough (discovery, register read, telemetry) lives in
[`examples/host_demo.rs`](./examples/host_demo.rs).

### End-to-end on an emulated MCU

The workspace member [`qemu-mps2/`](./qemu-mps2/) is a Cortex-M4 firmware demo that runs
under QEMU and reports temperature telemetry over UART; `qemu-mps2/examples/host_gateway.rs`
decodes those frames on the host. It doubles as the wire-protocol test harness:

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

## Design principles

1. **`#![no_std]` + `alloc` only** — no std, no tokio, no OS calls. The wire layer
   allocates frame buffers through `alloc`, so budget for a heap on the target rather than
   a fixed figure.
2. **Serde everywhere** — manifest types derive `Serialize`/`Deserialize`; wire payloads
   use [`postcard`](https://docs.rs/postcard), a compact no_std binary format.
3. **Encoding is pure, I/O belongs to the caller** — `Frame::encode` returns `Vec<u8>` and
   `decode_frame` returns a parsed `Frame`; the caller owns the transport (embassy UART,
   tokio serial, kernel IRQ) and the retry policy.
4. **Shared schema, independent implementations** — embassy nodes and evernight both
   deserialize the same `HardwareManifest`, but their runtime converters stay separate:
   a no_std node cannot depend on the gateway's runtime types or trait objects.

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
| `test` | host `x86_64`, all features |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, the primary embassy target) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (secondary target) |
| `fmt` | host, `cargo fmt --all --check` |
| `clippy` | host, `--all-features --all-targets -- -D warnings` |
| `bench-check` | host, `cargo bench --no-run` |

The CI gates run on the organization's self-hosted runners; the release workflow
(`.github/workflows/publish.yml`) runs on GitHub-hosted runners because this repository is
public. The gate definitions live in [`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

## Documentation

- API reference: [docs.rs/hare](https://docs.rs/hare)
- Translations: [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

Each `docs/<language>/README.md` is a translation; the English README is the source of
truth when they disagree.

## License

SySL-1.0 (Synthetic Source License) — see [LICENSE](./LICENSE).

---

## Provenance

The crate's source modules were extracted from the
[`kei`](https://github.com/celestia-island/kei) kernel repository (`packages/kei/`) on
2026-10-20 by user ruling, renamed `kei-wire` at creation and to `hare` (Omagari Hare,
小鉤晴) on 2026-10-21 by user ruling. The kernel keeps its freeze; this repository carries
the bridge crates (wire / manifest / codec / hal / ansi) forward. The README, the package
metadata, and the documentation were rewritten when the crate moved to its own repository.
