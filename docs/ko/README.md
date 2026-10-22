<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>Celestia IoT 생태계를 위한 <code>#![no_std]</code> 임베디드 브리지 라이브러리</strong></p>

<div align="center">

[![CI](https://github.com/celestia-island/hare/actions/workflows/ci.yml/badge.svg)](https://github.com/celestia-island/hare/actions/workflows/ci.yml)
[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![crates.io](https://img.shields.io/crates/v/hare)](https://crates.io/crates/hare)
[![docs.rs](https://img.shields.io/docsrs/hare)](https://docs.rs/hare)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zh-Hans/README.md) ·
[繁體中文](../zh-Hant/README.md) ·
[日本語](../ja/README.md) ·
**한국어** ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

Hare는 **embassy 기반 센서 노드**(베어메탈 MCU)와 **게이트웨이 브로커** — 현재 Linux에서
구동되는 [`evernight`](https://github.com/celestia-island/evernight) — 사이의 공유 계약
계층입니다. 양측은 동일한 하드웨어 매니페스트를 역직렬화하고 동일한 와이어 프레이밍을
사용하므로, 센서 펌웨어와 게이트웨이 서비스를 서로 독립적으로 개발하더라도 중간에서
맞물릴 수 있습니다.

`#![no_std]` + `alloc`으로 구성됩니다. std도, tokio도, OS 시스템 콜도 없으며 자체 비동기
런타임도 없습니다 — 전송 계층은 embassy 노드가 제공하고, 프로토콜은 hare가 제공합니다.

## 기능

- **매니페스트 스키마** (`hare::manifest`) — 레지스터 맵, 알람 규칙, 스테이션 설정, 스케일
  변환을 아우르는 하드웨어 기술. 양측이 동일한 스키마를 역직렬화합니다.
- **와이어 프로토콜** (`hare::wire`) — 센서 노드와 게이트웨이 사이의 UART/USB-CDC/SPI용
  컴팩트 바이너리 프레이밍 프로토콜: `0xCE` 매직, `u16` 길이, `u8` 메시지 타입,
  `postcard` 페이로드, CRC16-Modbus 트레일러.
- **프로토콜 타입 식별자** (`hare::codec`) — `ProtocolKind`와, Modbus, MC Protocol,
  EtherNet/IP-CIP, S7comm, CAN을 위한 불투명 `ProtocolFrame` 컨테이너. 인코드/디코드
  로직 자체는 hare가 아니라 evernight에 속합니다.
- **HAL 트레이트** (`hare::hal`) — embassy 노드가 자체 하드웨어에 대해 구현하는
  `Transport` 및 `SensorDevice` 트레이트.
- **ANSI/VT 파서** (`hare::ansi`) — 화면 모델을 보유하지 않고도 바이트 스트림을 이해해야
  하는 터미널과 프로토콜 로거를 위해, 의미 이벤트를 산출하는 상태 머신 방식 이스케이프
  시퀀스 파서.

## 설치

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false`는 embassy 펌웨어에서 옳은 선택입니다. 기본값은 no_std 전체
집합(`wire`, `manifest`, `codec`, `hal`, `ansi`)이며, 펌웨어는 보통 링크하는 모듈만
필요로 하기 때문입니다.

### 피처 플래그

| 피처 | 기본값 | 활성화되는 항목 |
|---|---|---|
| `wire` | ✅ | Celestia 와이어 프로토콜: `Node` / `Gateway` / `Frame`, postcard 페이로드, CRC16 |
| `manifest` | ✅ | 하드웨어 매니페스트 스키마 (`HardwareManifest`, 레지스터 맵, 알람, 스케일) |
| `codec` | ✅ | 프로토콜 타입 식별자와 불투명 `ProtocolFrame` 컨테이너 |
| `hal` | ✅ | `Transport` / `SensorDevice` 트레이트 — embassy에서 구현 가능 |
| `ansi` | ✅ | ANSI/VT 이스케이프 시퀀스 파서 (상태 머신, no_std) |
| `std` | ❌ | 호스트 빌드를 위해 선택적 `serde` / `postcard` 의존성으로 `std`를 전달 |

매니페스트는 평범한 serde 데이터입니다. hare는 TOML이나 JSON 어댑터를
**전혀 제공하지 않습니다**: 원하는 serde 포맷으로 로드하십시오(호스트 데모는
dev-dependency인 `serde_json`을 사용합니다).

## 빠른 시작

### 센서 노드 (embassy 펌웨어)

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

`Node`는 설계상 동기 방식입니다. 스스로 `Transport::recv`를 반복 호출하므로 어떤 전송
계층(embassy UART, RS-485 트랜시버, 테스트 파이프)과도 동작하며 런타임 의존성을 추가하지
않습니다. 전체 레퍼런스는
[`examples/embassy_node.rs`](../../examples/embassy_node.rs)에 있습니다 — 호스트에서는
스텁으로 컴파일되면서 실제 보드 배선을 주석 처리된 embassy 메인 루프에 담고 있는
템플릿입니다.

### 게이트웨이 측 (evernight)

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

디스커버리, 레지스터 읽기, 텔레메트리를 아우르는 호스트 전용 전체 워크스루는
[`examples/host_demo.rs`](../../examples/host_demo.rs)에 있습니다.

### 에뮬레이션된 MCU에서의 엔드투엔드

워크스페이스 멤버 [`qemu-mps2/`](../../qemu-mps2/)는 QEMU에서 실행되며 UART로 온도
텔레메트리를 보고하는 Cortex-M4 펌웨어 데모입니다. `qemu-mps2/examples/host_gateway.rs`가
그 프레임을 호스트에서 디코딩합니다. 이 구성은 와이어 프로토콜 테스트 하네스를 겸합니다:

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

## 설계 원칙

1. **`#![no_std]` + `alloc`만 사용** — std 없음, tokio 없음, OS 시스템 콜 없음. 와이어 계층은
   `alloc`을 통해 프레임 버퍼를 할당하므로, 고정된 수치가 아니라 대상 보드에서의 힙 예산을
   산정해야 합니다.
2. **Serde를 전면 사용** — 매니페스트 타입은 `Serialize`/`Deserialize`를 derive하고,
   와이어 페이로드는 컴팩트한 no_std 바이너리 포맷인
   [`postcard`](https://docs.rs/postcard)를 사용합니다.
3. **인코딩은 순수 함수, I/O는 호출자 소유** — `Frame::encode`는 `Vec<u8>`을 반환하고
   `decode_frame`은 파싱된 `Frame`을 반환합니다. 전송 계층(embassy UART, tokio 시리얼,
   커널 IRQ)과 재시도 정책은 호출자가 소유합니다.
4. **스키마는 공유, 구현은 독립** — embassy 노드와 evernight 모두 동일한
   `HardwareManifest`를 역직렬화하지만, 런타임 변환기는 각자 분리되어 있습니다: no_std
   노드는 게이트웨이의 런타임 타입이나 트레이트 객체에 의존할 수 없습니다.

## KEI 커널과의 관계

Hare는 원래 [`kei`](https://github.com/celestia-island/kei) 저장소의 `packages/kei/` 아래에
있던 것을 추출해 낸 것입니다. 커널은 **동결**되어 있으며([로드맵](https://docs.celestia.world)),
브리지 크레이트는 커널이 멈춰 있는 동안에도 계속 발전할 수 있도록 이제 이곳에 있습니다.

둘은 코드가 아니라 계약을 공유하는 독립적인 아티팩트입니다:

| | KEI 커널 (`celestia-island/kei`) | Hare (이 크레이트) |
|---|---|---|
| 대상 | ARM64 / RISC-V **게이트웨이**, Linux 시스템 콜 ABI, MMU 필요 | 베어메탈 **MCU**, MMU 없음 |
| 툴체인 | nightly, `aarch64-unknown-none` 계열 | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| 역할 | 게이트웨이에서 Linux를 대체할 장기 후보 | MCU 측 계약 |

이 저장소는 자체 완결적입니다. stable Rust, 자체 `rust-toolchain.toml`, 그리고 유일한
멤버(`qemu-mps2/`)가 이 디렉터리 안에 위치한 워크스페이스로 이루어져 있습니다. 클론하면
바로 동작하는 워크스페이스가 됩니다 — 커널 체크아웃도 `rustup` override도 필요하지 않습니다.

## 타깃과 CI

| 게이트 | 타깃 |
|---|---|
| `test` | 호스트 `x86_64`, 전체 피처 |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, 주 embassy 타깃) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (보조 타깃) |
| `fmt` | 호스트, `cargo fmt --all --check` |
| `clippy` | 호스트, `--all-features --all-targets -- -D warnings` |
| `bench-check` | 호스트, `cargo bench --no-run` |

CI 게이트는 조직의 셀프 호스팅 러너에서 실행됩니다. 릴리스 워크플로
(`.github/workflows/publish.yml`)는 이 저장소가 공개되어 있으므로 GitHub 호스팅 러너에서
실행됩니다. 게이트 정의는
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)에 있습니다.

## 문서

- API 레퍼런스: [docs.rs/hare](https://docs.rs/hare)
- 번역본: [简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

각 `docs/<language>/README.md`는 번역본이며, 내용이 서로 다를 경우 영어 README가 기준
원본입니다.

## 라이선스

SySL-1.0 (Synthetic Source License) — [LICENSE](../../LICENSE)를 참조하십시오.

---

## 출처

이 크레이트의 소스 모듈은 2026-10-20 사용자 결정에 따라
[`kei`](https://github.com/celestia-island/kei) 커널 저장소(`packages/kei/`)에서 추출되었고,
생성 당시 `kei-wire`였던 이름이 2026-10-21 사용자 결정에 따라 `hare`(Omagari Hare, 小鉤晴)로
변경되었습니다. 커널은 동결 상태를 그대로 유지하며, 이 저장소가 브리지 크레이트
(wire / manifest / codec / hal / ansi)를 이어서 발전시킵니다. README, 패키지 메타데이터,
그리고 문서는 크레이트가 자체 저장소로 옮겨질 때 다시 작성되었습니다.
