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

Hare es la capa de contrato compartida entre los **nodos de sensores basados en embassy**
(MCU bare-metal) y el **broker de la pasarela** — hoy
[`evernight`](https://github.com/celestia-island/evernight), ejecutándose sobre Linux. Ambos
lados deserializan el mismo manifiesto de hardware y hablan el mismo tramado de cable, así
que el firmware de un sensor y el servicio de la pasarela pueden construirse de forma
independiente y aun así encontrarse en el punto medio.

Es `#![no_std]` + `alloc`: sin std, sin tokio, sin llamadas al SO y sin un runtime asíncrono
propio — el nodo embassy proporciona el transporte y hare proporciona el protocolo.

## Características

- **Esquema de manifiesto** (`hare::manifest`) — descripciones de hardware: mapas de
  registros, reglas de alarma, configuraciones de estación y transformaciones de escala. El
  mismo esquema que deserializan ambos lados.
- **Protocolo de cable** (`hare::wire`) — un protocolo de tramado binario compacto para
  UART/USB-CDC/SPI entre los nodos de sensores y la pasarela: byte mágico `0xCE`, longitud
  `u16`, tipo de mensaje `u8`, carga útil `postcard` y tráiler CRC16-Modbus.
- **Identificadores de tipo de protocolo** (`hare::codec`) — `ProtocolKind` y un contenedor
  opaco `ProtocolFrame` que transporta una trama de fabricante a través de la capa de cable
  sin que hare la analice.
- **Concentrador de protocolos** (`hare::protocol`) — los propios activos de protocolo de la
  familia: CRC16 de Modbus y extracción de registros/bobinas, el tramado de petición/respuesta
  de MC Protocol 3E de Mitsubishi, y el modelo de áreas/parámetros de S7comm con informes
  DB → SCL. Los protocolos que necesitan una pila cliente completa de terceros permanecen en
  el servicio de la pasarela.
- **Rasgos HAL** (`hare::hal`) — los rasgos `Transport` y `SensorDevice` que los nodos embassy
  implementan contra su propio hardware.
- **Analizador ANSI/VT** (`hare::ansi`) — un analizador de secuencias de escape basado en una
  máquina de estados que emite eventos semánticos, para terminales y herramientas de registro
  de protocolo que necesitan entender un flujo de bytes sin tener que mantener un modelo de
  pantalla.

## Instalación

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false` es la opción correcta para un firmware embassy, pero las
características no son del todo independientes: `wire` arrastra `hal`, y los traits HAL
están escritos sobre los tipos de manifest y wire. Un firmware suele activar el conjunto
completo (o `wire,manifest`) en lugar de un módulo suelto.

### Banderas de características

| Característica | Por defecto | Qué habilita |
|---|---|---|
| `wire` | ✅ | Protocolo de cable de Celestia: `Node` / `Gateway` / `Frame`, cargas útiles postcard, CRC16 |
| `manifest` | ✅ | Esquema del manifiesto de hardware (`HardwareManifest`, mapas de registros, alarmas, escalas) |
| `codec` | ✅ | Identificadores de tipo de protocolo y el contenedor opaco `ProtocolFrame` |
| `hal` | ✅ | Rasgos `Transport` / `SensorDevice` — implementables por embassy |
| `ansi` | ✅ | Analizador de secuencias de escape ANSI/VT (máquina de estados, no_std) |
| `std` | ❌ | Reenvía `std` a las dependencias opcionales `serde` / `postcard` para compilaciones en el host |

`hare::protocol` se compila siempre: no añade dependencias.

El manifiesto es simple dato serde. hare **no** incluye adaptadores TOML ni JSON: cárguelo
con el formato serde que prefiera.

## Inicio rápido

### Nodo sensor (firmware embassy)

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

`Node` es síncrono por diseño: itera sobre `Transport::recv` por sí mismo, así que funciona
con cualquier transporte (UART de embassy, transceptor RS-485, una tubería de prueba) y no
añade ninguna dependencia de runtime. La referencia completa está en
[`examples/embassy_node.rs`](../../examples/embassy_node.rs) — una plantilla que compila en el
host como stub y que lleva el cableado real de la placa en su bucle principal de embassy
comentado.

### Lado de la pasarela (evernight)

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

Un recorrido completo exclusivo del host (descubrimiento, lectura de registros, telemetría)
está en [`examples/host_demo.rs`](../../examples/host_demo.rs).

### De extremo a extremo en una MCU emulada

El miembro del workspace [`qemu-mps2/`](../../qemu-mps2/) es una demo de firmware para
Cortex-M4 que se ejecuta bajo QEMU y reporta telemetría de temperatura por UART;
`qemu-mps2/examples/host_gateway.rs` decodifica esas tramas en el host. Además sirve como
banco de pruebas del protocolo de cable:

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

## Principios de diseño

1. **Solo `#![no_std]` + `alloc`** — sin std, sin tokio, sin llamadas al SO. La capa de cable
   asigna los búferes de trama mediante `alloc`, así que hay que presupuestar un heap en el
   destino en lugar de una cifra fija.
2. **Serde en todas partes** — los tipos del manifiesto derivan `Serialize`/`Deserialize`; las
   cargas útiles del cable usan [`postcard`](https://docs.rs/postcard), un formato binario
   compacto para no_std.
3. **La codificación es pura, la E/S pertenece al llamador** — `Frame::encode` devuelve
   `Vec<u8>` y `decode_frame` devuelve un `Frame` ya analizado; el llamador es responsable del
   transporte (UART de embassy, puerto serie de tokio, IRQ del kernel) y de la política de
   reintentos.
4. **Esquema compartido, implementaciones independientes** — los nodos embassy y evernight
   deserializan el mismo `HardwareManifest`, pero sus conversores de runtime permanecen
   separados: un nodo no_std no puede depender de los tipos de runtime ni de los objetos trait
   de la pasarela.

## Relación con el kernel KEI

Hare se extrajo del repositorio [`kei`](https://github.com/celestia-island/kei), donde vivía
bajo `packages/kei/`. El kernel está **congelado**
([hoja de ruta](https://docs.celestia.world)), y el crate puente vive ahora aquí para poder
seguir evolucionando mientras el kernel no lo hace.

Ambos son artefactos independientes que comparten un contrato en lugar de código:

| | Kernel KEI (`celestia-island/kei`) | Hare (este crate) |
|---|---|---|
| Plataforma objetivo | **pasarelas** ARM64 / RISC-V, ABI de llamadas al sistema de Linux, MMU obligatoria | **MCU** bare-metal, sin MMU |
| Cadena de herramientas | nightly, familia `aarch64-unknown-none` | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| Rol | candidato a largo plazo para sustituir a Linux en la pasarela | el contrato del lado de la MCU |

Este repositorio es autocontenido: Rust estable, su propio `rust-toolchain.toml` y un
workspace cuyo único miembro (`qemu-mps2/`) vive dentro de este directorio. Clonarlo te da un
workspace funcional — no se necesita ni un checkout del kernel ni ningún override de `rustup`.

## Plataformas objetivo y CI

| Puerta | Objetivo |
|---|---|
| `test` | host `x86_64`, todas las características |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, el objetivo principal de embassy) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (objetivo secundario) |
| `fmt` | host, `cargo fmt --all --check` |
| `clippy` | host, `--all-features --all-targets -- -D warnings` |
| `bench-check` | host, `cargo bench --no-run` |

Las puertas de CI se ejecutan en los runners autoalojados de la organización; el flujo de
trabajo de publicación (`.github/workflows/publish.yml`) se ejecuta en runners alojados en
GitHub porque este repositorio es público. Las definiciones de las puertas viven en
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

## Documentación

- Referencia de la API: [docs.rs/hare](https://docs.rs/hare)
- Traducciones: [简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

Cada `docs/<language>/README.md` es una traducción; el README en inglés es la fuente de
verdad cuando discrepan.

## Licencia

SySL-1.0 (Synthetic Source License) — consulta [LICENSE](../../LICENSE).

---

## Procedencia

Los módulos fuente del crate se extrajeron del repositorio del kernel
[`kei`](https://github.com/celestia-island/kei) (`packages/kei/`) el 2026-10-20 por decisión
del usuario; se renombraron a `kei-wire` en su creación y a `hare` (Omagari Hare, 小鉤晴) el
2026-10-21 por decisión del usuario. El kernel sigue congelado; este repositorio hace
avanzar los crates puente (wire / manifest / codec / hal / ansi). El README, los metadatos del
paquete y la documentación se reescribieron cuando el crate pasó a su propio repositorio.
