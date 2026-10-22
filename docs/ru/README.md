<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>Библиотека-мост с <code>#![no_std]</code> для встраиваемых систем экосистемы Celestia IoT</strong></p>

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
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
**Русский** ·
[العربية](../ar/README.md)

</div>

Hare — это общий контрактный слой между **сенсорными узлами на базе embassy** (микроконтроллеры без ОС) и **брокером шлюза** — сегодня это [`evernight`](https://github.com/celestia-island/evernight), работающий под Linux. Обе стороны десериализуют один и тот же манифест оборудования и используют одинаковый формат кадров при обмене по линии связи, поэтому прошивку датчика и сервис шлюза можно разрабатывать независимо — они всё равно будут совместимы.

Библиотека — это `#![no_std]` + `alloc`: без std, без tokio, без системных вызовов и без собственного асинхронного рантайма — узел embassy предоставляет транспорт, а hare — протокол.

## Возможности

- **Схема манифеста** (`hare::manifest`) — описания оборудования: карты регистров, правила аварийной сигнализации, конфигурации станций и преобразования шкал. Обе стороны десериализуют одну и ту же схему.
- **Протокол передачи по линии связи** (`hare::wire`) — компактный бинарный протокол кадрирования для UART/USB-CDC/SPI между сенсорными узлами и шлюзом: магическое число `0xCE`, длина `u16`, полезная нагрузка `postcard`, завершающее поле CRC16-Modbus.
- **Идентификаторы типов протоколов** (`hare::codec`) — `ProtocolKind` и непрозрачный контейнер `ProtocolFrame` для Modbus, MC Protocol, EtherNet/IP-CIP, S7comm и CAN. Сама логика кодирования и декодирования относится к evernight, а не к hare.
- **Трейты HAL** (`hare::hal`) — `Transport` и `SensorDevice`, которые узлы embassy реализуют для собственной аппаратуры.
- **Парсер ANSI/VT** (`hare::ansi`) — конечный автомат для разбора escape-последовательностей, порождающий семантические события; предназначен для терминалов и средств журналирования протоколов, которым нужно понимать поток байтов, не имея модели экрана.

## Установка

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false` — правильный выбор для прошивки embassy: по умолчанию включён полный набор no_std (`wire`, `manifest`, `codec`, `hal`, `ansi`), а прошивке обычно нужны только те модули, которые она подключает.

### Флаги возможностей

| Флаг | По умолчанию | Что включает |
|---|---|---|
| `wire` | ✅ | Протокол обмена Celestia: `Node` / `Gateway` / `Frame`, полезные данные postcard, CRC16 |
| `manifest` | ✅ | Схема манифеста оборудования (`HardwareManifest`, карты регистров, аварийные сигналы, шкалы) |
| `codec` | ✅ | Идентификаторы типов протоколов и непрозрачный контейнер `ProtocolFrame` |
| `hal` | ✅ | Трейты `Transport` / `SensorDevice` — реализуемые в embassy |
| `ansi` | ✅ | Парсер escape-последовательностей ANSI/VT (конечный автомат, no_std) |
| `std` | ❌ | Вспомогательные средства только для std (адаптеры манифестов TOML/JSON) на стороне хоста |

## Быстрый старт

### Сенсорный узел (прошивка embassy)

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

Полный справочный пример находится в [`examples/embassy_node.rs`](./examples/embassy_node.rs); он не собирается на хосте (требуется HAL платы) и предназначен для копирования в проект прошивки.

### Сторона шлюза (evernight)

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

Сквозная демонстрация, работающая только на хосте, находится в [`examples/host_demo.rs`](./examples/host_demo.rs).

### Сквозной пример на эмулируемом MCU

Компонент рабочей области [`qemu-mps2/`](./qemu-mps2/) — это демонстрационная прошивка для Cortex-M4, которая работает под управлением QEMU и передаёт телеметрию температуры по UART; файл `examples/host_gateway.rs` в том же каталоге декодирует эти кадры на хосте. Он также служит тестовым стендом протокола обмена:

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## Принципы проектирования

1. **Только `#![no_std]` + `alloc`** — без std, без tokio, без системных вызовов. Слой обмена выделяет буферы кадров через `alloc`, поэтому на целевой платформе следует рассчитывать на кучу, а не на фиксированный объём памяти.
2. **Serde повсюду** — для типов манифеста выводятся реализации `Serialize`/`Deserialize`; полезные данные протокола используют [`postcard`](https://docs.rs/postcard) — компактный бинарный формат no_std.
3. **Кодек ≠ транспорт** — кодирование и декодирование — чистые функции, возвращающие `Vec<u8>`; за ввод-вывод отвечает вызывающая сторона (асинхронный код embassy, tokio, прерывания ядра — всё подходит).
4. **Общая схема, независимые реализации** — узлы embassy и evernight десериализуют один и тот же `HardwareManifest`, но их преобразователи времени выполнения остаются раздельными (в no_std нельзя использовать замыкания или `Arc<dyn Fn>`).

## Связь с ядром KEI

Hare был выделен из репозитория [`kei`](https://github.com/celestia-island/kei), где он находился в `packages/kei/`. Ядро **заморожено** ([дорожная карта](https://docs.celestia.world)), и крейт-мост теперь находится здесь, чтобы продолжать развиваться, пока ядро остаётся неизменным.

Это два независимых артефакта, у которых общий контракт, а не общий код:

| | Ядро KEI (`celestia-island/kei`) | Hare (этот крейт) |
|---|---|---|
| Цель | ARM64 / RISC-V **шлюзы**, ABI системных вызовов Linux, требуется MMU | **MCU** без ОС, MMU не требуется |
| Тулчейн | nightly, семейство `aarch64-unknown-none` | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| Роль | долгосрочный кандидат на замену Linux на шлюзе | контракт на стороне MCU |

Этот репозиторий самодостаточен: стабильный Rust, собственный `rust-toolchain.toml` и рабочая область, единственный член которой (`qemu-mps2/`) находится в этом каталоге. После клонирования вы получаете готовую рабочую область — не нужно ни клонировать ядро, ни настраивать переопределение в `rustup`.

## Целевые платформы и CI

| Проверка | Целевая платформа |
|---|---|
| `test` | хост (`x86_64`), все возможности |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, основная целевая платформа embassy) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (дополнительная целевая платформа) |
| `fmt` / `clippy` / `bench-check` | хост, `--all-targets` |

Все проверки выполняются на собственных раннерах организации; файл рабочего процесса — [`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

## Документация

- Справочник по API: [docs.rs/hare](https://docs.rs/hare)
- Переводы: [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

Все перечисленные версии, включая эту, являются переводами. Английский README — исходный текст и источник истины.

## Лицензия

SySL-1.0 (Synthetic Source License) — см. [LICENSE](./LICENSE).

---

## Происхождение

Дословно извлечено из репозитория ядра [`kei`](https://github.com/celestia-island/kei) (`packages/kei/`) 2026-10-20 по решению пользователя; при создании переименовано в `kei-wire`, а 2026-10-21 по решению пользователя — в `hare` (Omagari Hare, 小鉤晴). Ядро остаётся замороженным; этот репозиторий продолжает развивать крейты-мосты (wire / manifest / codec / hal).
