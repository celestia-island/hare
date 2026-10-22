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
계층입니다. 양쪽 모두 동일한 하드웨어 매니페스트를 역직렬화하고 동일한 와이어 프레이밍을
사용하므로, 센서 펌웨어와 게이트웨이 서비스를 서로 독립적으로 개발하더라도 중간에서
맞물릴 수 있습니다.

`#![no_std]` + `alloc`으로 구성됩니다. std 없음, tokio 없음, OS 호출 없음, 자체 비동기
런타임도 없습니다 — 전송 계층은 embassy 노드가 제공하고, hare는 프로토콜을 제공합니다.

## 기능

- **매니페스트 스키마** (`hare::manifest`) — 하드웨어 기술: 레지스터 맵, 알람 규칙,
  스테이션 설정, 스케일 변환. 양쪽이 동일한 스키마를 역직렬화합니다.
- **와이어 프로토콜** (`hare::wire`) — 센서 노드와 게이트웨이 사이의 UART/USB-CDC/SPI용
  컴팩트 바이너리 프레이밍 프로토콜: `0xCE` 매직, `u16` 길이, `postcard` 페이로드,
  CRC16-Modbus 트레일러.
- **프로토콜 타입 식별자** (`hare::codec`) — `ProtocolKind`, 그리고 Modbus, MC Protocol,
  EtherNet/IP-CIP, S7comm, CAN을 위한 불투명 컨테이너 `ProtocolFrame`. 인코드/디코드
  로직 자체는 hare가 아니라 evernight에 속합니다.
- **HAL 트레이트** (`hare::hal`) — embassy 노드가 자체 하드웨어에 대해 구현하는
  `Transport` 및 `SensorDevice` 트레이트.
- **ANSI/VT 파서** (`hare::ansi`) — 의미 이벤트를 산출하는 상태 머신 방식 이스케이프
  시퀀스 파서로, 화면 모델을 보유하지 않고도 바이트 스트림을 이해해야 하는 터미널과
  프로토콜 로거를 위한 것입니다.

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
| `std` | ❌ | 호스트 측을 위한 std 전용 헬퍼 (TOML/JSON 매니페스트 어댑터) |

## 빠른 시작

### 센서 노드 (embassy 펌웨어)

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

전체 레퍼런스는 [`examples/embassy_node.rs`](./examples/embassy_node.rs)에 있습니다. 이
예제는 보드 HAL이 필요하므로 호스트에서는 컴파일되지 않으며, 펌웨어 프로젝트로 복사해
사용하는 것을 전제로 합니다.

### 게이트웨이 측 (evernight)

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

호스트 전용 엔드투엔드 데모는 [`examples/host_demo.rs`](./examples/host_demo.rs)에 있습니다.

### 에뮬레이션된 MCU에서의 엔드투엔드

워크스페이스 멤버 [`qemu-mps2/`](./qemu-mps2/)는 QEMU에서 실행되며 UART로 온도
텔레메트리를 보고하는 Cortex-M4 펌웨어 데모입니다. 같은 디렉터리의
`examples/host_gateway.rs`가 그 프레임을 호스트에서 디코딩합니다. 이 구성은 와이어
프로토콜 테스트 하네스를 겸합니다:

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## 설계 원칙

1. **`#![no_std]` + `alloc`만 사용** — std 없음, tokio 없음, OS 호출 없음. 와이어 계층은
   `alloc`을 통해 프레임 버퍼를 할당하므로, 고정된 수치 대신 대상 보드에서의 힙 예산을
   산정해야 합니다.
2. **Serde를 전면 사용** — 매니페스트 타입은 `Serialize`/`Deserialize`를 derive하고,
   와이어 페이로드는 컴팩트한 no_std 바이너리 포맷인
   [`postcard`](https://docs.rs/postcard)를 사용합니다.
3. **코덱 ≠ 전송** — 인코드/디코드는 `Vec<u8>`을 반환하는 순수 함수이며, I/O는 호출자가
   소유합니다 (embassy async, tokio, 커널 IRQ — 모두 동작합니다).
4. **스키마는 공유, 구현은 독립** — embassy 노드와 evernight 모두 동일한
   `HardwareManifest`를 역직렬화하지만, 런타임 변환기는 각자 분리되어 있습니다
   (no_std에서는 클로저나 `Arc<dyn Fn>`을 사용할 수 없습니다).

## KEI 커널과의 관계

Hare는 원래 [`kei`](https://github.com/celestia-island/kei) 저장소의 `packages/kei/`
아래에 있던 것을 추출해 낸 것입니다. 커널은 **동결**되어 있으며([로드맵](https://docs.celestia.world)),
브리지 크레이트는 커널이 멈춰 있는 동안에도 계속 발전할 수 있도록 이제 이곳에 있습니다.

둘은 코드가 아니라 계약을 공유하는 독립적인 아티팩트입니다:

| | KEI 커널 (`celestia-island/kei`) | Hare (이 크레이트) |
|---|---|---|
| 대상 | ARM64 / RISC-V **게이트웨이**, Linux syscall ABI, MMU 필요 | 베어메탈 **MCU**, MMU 없음 |
| 툴체인 | nightly, `aarch64-unknown-none` 계열 | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| 역할 | 게이트웨이에서 Linux를 대체할 장기 후보 | MCU 측 계약 |

이 저장소는 자체 완결적입니다. stable Rust, 자체 `rust-toolchain.toml`, 그리고 유일한
멤버(`qemu-mps2/`)가 이 디렉터리 안에 위치한 워크스페이스로 이루어져 있습니다. 클론하면
바로 동작하는 워크스페이스가 되며, 커널 체크아웃도 `rustup` override도 필요하지 않습니다.

## 타깃과 CI

| 게이트 | 타깃 |
|---|---|
| `test` | 호스트 (`x86_64`), 전체 피처 |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, 주 embassy 타깃) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (보조 타깃) |
| `fmt` / `clippy` / `bench-check` | 호스트, `--all-targets` |

모든 작업은 조직의 셀프 호스팅 러너에서 실행되며, 워크플로는
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml)입니다.

## 문서

- API 레퍼런스: [docs.rs/hare](https://docs.rs/hare)
- 번역본: [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

이 문서를 포함한 각 언어 문서는 영어 README의 번역본이며, 내용상 기준 원본(source of
truth)은 영어 README입니다.

## 라이선스

SySL-1.0 (Synthetic Source License) — [LICENSE](./LICENSE)를 참조하십시오.

---

## 출처

2026-10-20 사용자 결정에 따라 [`kei`](https://github.com/celestia-island/kei) 커널
저장소(`packages/kei/`)에서 그대로 추출되었고, 생성 당시 `kei-wire`였던 이름이
2026-10-21 사용자 결정에 따라 `hare`(Omagari Hare, 小鉤晴)로 변경되었습니다. 커널은
동결 상태를 유지하며, 이 저장소가 브리지 크레이트(wire / manifest / codec / hal)를
이어서 발전시킵니다.
