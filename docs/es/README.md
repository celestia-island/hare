<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>Una biblioteca puente embebida <code>#![no_std]</code> para el ecosistema IoT de Celestia</strong></p>

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
**Español** ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

Hare es la capa de contrato compartida entre **los nodos de sensores basados en embassy**
(MCU bare-metal) y **el broker de la pasarela** — hoy
[`evernight`](https://github.com/celestia-island/evernight), ejecutándose sobre Linux. Ambos
lados deserializan el mismo manifiesto de hardware y usan el mismo formato de trama sobre el
cable, de modo que el firmware de un sensor y el servicio de la pasarela pueden
desarrollarse de forma independiente y aun así encontrarse en el punto medio.

Es `#![no_std]` + `alloc`: sin std, sin tokio, sin llamadas al SO y sin un runtime asíncrono
propio — el nodo embassy proporciona el transporte y hare proporciona el protocolo.

## Características

- **Esquema de manifiesto** (`hare::manifest`) — descripciones de hardware: mapas de
  registros, reglas de alarma, configuraciones de estación y transformaciones de escala. El
  mismo esquema que ambos lados deserializan.
- **Protocolo de cable** (`hare::wire`) — un protocolo de entramado binario compacto para
  UART/USB-CDC/SPI entre los nodos de sensores y la pasarela: valor mágico `0xCE`, longitud
  `u16`, carga útil `postcard` y tráiler CRC16-Modbus.
- **Identificadores de tipo de protocolo** (`hare::codec`) — `ProtocolKind` y un contenedor
  opaco `ProtocolFrame` para Modbus, MC Protocol, EtherNet/IP-CIP, S7comm y CAN. La lógica de
  codificación y decodificación pertenece a evernight, no a hare.
- **Rasgos HAL** (`hare::hal`) — los rasgos `Transport` y `SensorDevice` que los nodos embassy
  implementan para su propio hardware.
- **Analizador ANSI/VT** (`hare::ansi`) — un analizador de secuencias de escape basado en una
  máquina de estados que emite eventos semánticos, para terminales y registradores de
  protocolo que necesitan entender un flujo de bytes sin mantener un modelo de pantalla.

## Instalación

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false` es la opción correcta para el firmware de un nodo embassy: los
valores por defecto son el conjunto no_std completo (`wire`, `manifest`, `codec`, `hal`,
`ansi`), y un firmware normalmente solo necesita los módulos que enlaza.

### Banderas de características

| Característica | Por defecto | Qué habilita |
|---|---|---|
| `wire` | ✅ | Protocolo de cable de Celestia: `Node` / `Gateway` / `Frame`, cargas útiles postcard, CRC16 |
| `manifest` | ✅ | Esquema del manifiesto de hardware (`HardwareManifest`, mapas de registros, alarmas, escalas) |
| `codec` | ✅ | Identificadores de tipo de protocolo y el contenedor opaco `ProtocolFrame` |
| `hal` | ✅ | Rasgos `Transport` / `SensorDevice` — implementables por embassy |
| `ansi` | ✅ | Analizador de secuencias de escape ANSI/VT (máquina de estados, no_std) |
| `std` | ❌ | Utilidades exclusivas de std (adaptadores de manifiesto TOML/JSON) del lado del host |

## Inicio rápido

### Nodo sensor (firmware embassy)

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

La referencia completa está en [`examples/embassy_node.rs`](./examples/embassy_node.rs); no
compila en el host (necesita el HAL de una placa) y está pensada para copiarse en un proyecto
de firmware.

### Lado de la pasarela (evernight)

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

Hay una demo de extremo a extremo exclusiva del host en
[`examples/host_demo.rs`](./examples/host_demo.rs).

### Extremo a extremo en una MCU emulada

El miembro del workspace [`qemu-mps2/`](./qemu-mps2/) es una demo de firmware para Cortex-M4
que se ejecuta bajo QEMU y reporta telemetría de temperatura por UART;
`examples/host_gateway.rs`, en el mismo directorio, decodifica esas tramas en el host. Además
sirve como banco de pruebas del protocolo de cable:

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## Principios de diseño

1. **Solo `#![no_std]` + `alloc`** — sin std, sin tokio, sin llamadas al SO. La capa de cable
   asigna los búferes de trama mediante `alloc`, así que hay que dimensionar el heap del
   dispositivo de destino en lugar de asumir una cifra fija.
2. **Serde en todas partes** — los tipos del manifiesto derivan `Serialize`/`Deserialize`; las
   cargas útiles del cable usan [`postcard`](https://docs.rs/postcard), un formato binario
   compacto para no_std.
3. **Códec ≠ transporte** — codificar y decodificar son funciones puras que devuelven
   `Vec<u8>`; el llamador gestiona la E/S (async de embassy, tokio, IRQ del kernel — todo
   funciona).
4. **Esquema compartido, implementaciones independientes** — los nodos embassy y evernight
   deserializan el mismo `HardwareManifest`, pero sus conversores de tiempo de ejecución
   siguen separados (no_std no puede usar cierres ni `Arc<dyn Fn>`).

## Relación con el kernel KEI

Hare se extrajo del repositorio [`kei`](https://github.com/celestia-island/kei), donde vivía
bajo `packages/kei/`. El kernel está **congelado**
([hoja de ruta](https://docs.celestia.world)), y el crate puente ahora vive aquí para poder
seguir evolucionando mientras el kernel no lo hace.

Ambos son artefactos independientes que comparten un contrato en lugar de código:

| | Kernel KEI (`celestia-island/kei`) | Hare (este crate) |
|---|---|---|
| Plataforma objetivo | **pasarelas** ARM64 / RISC-V, ABI de llamadas al sistema de Linux, MMU obligatoria | **MCU** bare-metal, sin MMU |
| Cadena de herramientas | nightly, familia `aarch64-unknown-none` | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| Rol | candidato a largo plazo para sustituir a Linux en la pasarela | el contrato del lado de la MCU |

Este repositorio es autocontenido: Rust estable, su propio `rust-toolchain.toml` y un
workspace cuyo único miembro (`qemu-mps2/`) vive dentro de este mismo directorio. Clonarlo te
da un workspace funcional: no necesitas clonar el kernel ni ningún override de `rustup`.

## Plataformas objetivo y CI

| Comprobación | Plataforma objetivo |
|---|---|
| `test` | host (`x86_64`), todas las características |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, el objetivo principal de embassy) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (objetivo secundario) |
| `fmt` / `clippy` / `bench-check` | host, `--all-targets` |

Todo se ejecuta en los runners autoalojados de la organización; el flujo de trabajo es
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

## Documentación

- Referencia de la API: [docs.rs/hare](https://docs.rs/hare)
- Traducciones: [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

Todas las traducciones anteriores son versiones del README en inglés, que es la fuente de
verdad y prevalece ante cualquier discrepancia.

## Licencia

SySL-1.0 (Synthetic Source License) — consulta [LICENSE](./LICENSE).

---

## Procedencia

Extraído textualmente del repositorio del kernel [`kei`](https://github.com/celestia-island/kei)
(`packages/kei/`) el 2026-10-20 por decisión del usuario; se renombró a `kei-wire` en su
creación y a `hare` (Omagari Hare, 小鉤晴) el 2026-10-21 por decisión del usuario. El kernel
mantiene su congelación; este repositorio sigue adelante con los crates puente (wire /
manifest / codec / hal).
