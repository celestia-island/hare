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

Hare — это общий контрактный слой между **сенсорными узлами на базе embassy** (микроконтроллерами bare-metal) и **брокером шлюза** — сегодня это [`evernight`](https://github.com/celestia-island/evernight), работающий под Linux. Обе стороны десериализуют один и тот же манифест оборудования и используют одно и то же кадрирование на линии связи, поэтому прошивку датчика и сервис шлюза можно разрабатывать независимо, и они всё равно сойдутся посередине.

Библиотека построена на `#![no_std]` + `alloc`: без std, без tokio, без системных вызовов и без собственного асинхронного рантайма — транспорт предоставляет узел embassy, а протокол — hare.

## Возможности

- **Схема манифеста** (`hare::manifest`) — описания оборудования: карты регистров, правила аварийной сигнализации, конфигурации станций и преобразования шкал. Одну и ту же схему десериализуют обе стороны.
- **Протокол линии связи** (`hare::wire`) — компактный бинарный протокол кадрирования для UART/USB-CDC/SPI между сенсорными узлами и шлюзом: магическое число `0xCE`, длина `u16`, тип сообщения `u8`, полезная нагрузка `postcard`, завершающее поле CRC16-Modbus.
- **Идентификаторы типов протоколов** (`hare::codec`) — `ProtocolKind` и непрозрачный контейнер `ProtocolFrame` для Modbus, MC Protocol, EtherNet/IP-CIP, S7comm и CAN. Сама логика кодирования и декодирования принадлежит evernight, а не hare.
- **Трейты HAL** (`hare::hal`) — трейты `Transport` и `SensorDevice`, которые узлы embassy реализуют применительно к собственной аппаратуре.
- **Парсер ANSI/VT** (`hare::ansi`) — конечный автомат разбора escape-последовательностей, порождающий семантические события; он предназначен для терминалов и средств журналирования протоколов, которым нужно понимать поток байтов, не владея моделью экрана.

## Установка

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false` — правильный выбор для прошивки embassy: по умолчанию включён полный набор no_std (`wire`, `manifest`, `codec`, `hal`, `ansi`), а прошивке обычно нужны только те модули, которые она подключает.

### Флаги возможностей

| Флаг | По умолчанию | Что включает |
|---|---|---|
| `wire` | ✅ | Протокол линии связи Celestia: `Node` / `Gateway` / `Frame`, полезные нагрузки postcard, CRC16 |
| `manifest` | ✅ | Схема манифеста оборудования (`HardwareManifest`, карты регистров, аварийные сигналы, шкалы) |
| `codec` | ✅ | Идентификаторы типов протоколов и непрозрачный контейнер `ProtocolFrame` |
| `hal` | ✅ | Трейты `Transport` / `SensorDevice` — реализуемые в embassy |
| `ansi` | ✅ | Парсер escape-последовательностей ANSI/VT (конечный автомат, no_std) |
| `std` | ❌ | Пробрасывает `std` в необязательные зависимости `serde` / `postcard` при сборке для хоста |

Манифест — это обычные данные serde. hare **не** поставляет адаптеров TOML или JSON: загружайте его в любом удобном для вас формате serde (в демонстрационном примере для хоста используется `serde_json`, dev-зависимость).

## Быстрый старт

### Сенсорный узел (прошивка embassy)

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

`Node` намеренно синхронный: он сам опрашивает `Transport::recv` в цикле, поэтому работает с любым транспортом (UART embassy, трансивер RS-485, тестовый канал) и не добавляет зависимостей от рантайма. Полный справочный пример находится в [`examples/embassy_node.rs`](../../examples/embassy_node.rs) — это шаблон, который на хосте собирается как заглушка и содержит реальную обвязку платы в закомментированном главном цикле embassy.

### Сторона шлюза (evernight)

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

Полный сквозной пример, работающий только на хосте (обнаружение, чтение регистра, телеметрия), находится в [`examples/host_demo.rs`](../../examples/host_demo.rs).

### Сквозной пример на эмулируемом MCU

Элемент рабочей области [`qemu-mps2/`](../../qemu-mps2/) — это демонстрационная прошивка для Cortex-M4, которая работает под QEMU и передаёт телеметрию температуры по UART; файл `qemu-mps2/examples/host_gateway.rs` декодирует эти кадры на хосте. Заодно он служит тестовым стендом протокола линии связи:

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

## Принципы проектирования

1. **Только `#![no_std]` + `alloc`** — без std, без tokio, без системных вызовов. Слой линии связи выделяет буферы кадров через `alloc`, поэтому на целевой платформе следует предусматривать кучу, а не фиксированный объём памяти.
2. **Serde повсюду** — типы манифеста выводят `Serialize`/`Deserialize`; полезные нагрузки линии связи используют [`postcard`](https://docs.rs/postcard) — компактный бинарный формат no_std.
3. **Кодирование — чистая функция, ввод-вывод принадлежит вызывающей стороне** — `Frame::encode` возвращает `Vec<u8>`, а `decode_frame` — разобранный `Frame`; транспорт (UART embassy, последовательный порт tokio, прерывание ядра) и политику повторных попыток определяет вызывающая сторона.
4. **Общая схема, независимые реализации** — узлы embassy и evernight десериализуют один и тот же `HardwareManifest`, но их преобразователи времени выполнения остаются раздельными: узел no_std не может зависеть от типов времени выполнения шлюза или от трейт-объектов.

## Связь с ядром KEI

Hare был выделен из репозитория [`kei`](https://github.com/celestia-island/kei), где он находился в каталоге `packages/kei/`. Ядро **заморожено** ([дорожная карта](https://docs.celestia.world)), и крейт-мост теперь живёт здесь, чтобы продолжать развиваться, пока ядро остаётся неизменным.

Это два независимых артефакта, которые разделяют контракт, а не код:

| | Ядро KEI (`celestia-island/kei`) | Hare (этот крейт) |
|---|---|---|
| Целевая платформа | ARM64 / RISC-V **шлюзы**, ABI системных вызовов Linux, требуется MMU | **микроконтроллеры** bare-metal, без MMU |
| Тулчейн | nightly, семейство `aarch64-unknown-none` | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| Роль | долгосрочный кандидат на замену Linux на шлюзе | контракт на стороне MCU |

Этот репозиторий самодостаточен: стабильный Rust, собственный `rust-toolchain.toml` и рабочая область, единственный элемент которой (`qemu-mps2/`) находится в этом каталоге. После клонирования вы получаете готовую к работе рабочую область — не нужно ни клонировать ядро, ни настраивать переопределение в `rustup`.

## Целевые платформы и CI

| Проверка | Целевая платформа |
|---|---|
| `test` | хост `x86_64`, все возможности |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, основная целевая платформа embassy) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (дополнительная целевая платформа) |
| `fmt` | хост, `cargo fmt --all --check` |
| `clippy` | хост, `--all-features --all-targets -- -D warnings` |
| `bench-check` | хост, `cargo bench --no-run` |

Проверки CI выполняются на собственных раннерах организации; рабочий процесс выпуска (`.github/workflows/publish.yml`) выполняется на раннерах, предоставленных GitHub, поскольку этот репозиторий публичный. Определения проверок находятся в [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

## Документация

- Справочник по API: [docs.rs/hare](https://docs.rs/hare)
- Переводы: [简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

Каждый файл `docs/<language>/README.md` — это перевод; при расхождениях источником истины считается английский README.

## Лицензия

SySL-1.0 (Synthetic Source License) — см. [LICENSE](../../LICENSE).

---

## Происхождение

Исходные модули крейта были извлечены из репозитория ядра [`kei`](https://github.com/celestia-island/kei) (`packages/kei/`) 2026-10-20 по решению пользователя, при создании переименованы в `kei-wire`, а 2026-10-21 по решению пользователя — в `hare` (Omagari Hare, 小鉤晴). Ядро сохраняет свою заморозку; этот репозиторий продолжает развивать крейты-мосты (wire / manifest / codec / hal / ansi). README, метаданные пакета и документация были переписаны, когда крейт переехал в собственный репозиторий.
