<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>Une bibliothèque-pont embarquée <code>#![no_std]</code> pour l'écosystème IoT de Celestia</strong></p>

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
**Français** ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

Hare est la couche de contrat partagée entre les **nœuds capteurs à base d'embassy**
(MCU bare-metal) et le **broker de passerelle** — aujourd'hui
[`evernight`](https://github.com/celestia-island/evernight) sous Linux. Les deux côtés
désérialisent le même manifeste matériel et parlent le même tramage de liaison, si bien
qu'un firmware de capteur et un service de passerelle peuvent être développés
indépendamment et parvenir malgré tout à se rejoindre.

Il est `#![no_std]` + `alloc` : ni std, ni tokio, ni appels système, et aucun runtime
asynchrone propre — le nœud embassy fournit le transport, hare fournit le protocole.

## Fonctionnalités

- **Schéma de manifeste** (`hare::manifest`) — descriptions matérielles : cartes de
  registres, règles d'alarme, configurations de station et transformations d'échelle. Le
  même schéma que désérialisent les deux côtés.
- **Protocole de liaison** (`hare::wire`) — un protocole de tramage binaire compact pour
  l'UART/USB-CDC/SPI entre les nœuds capteurs et la passerelle : magie `0xCE`, longueur
  `u16`, type de message `u8`, charge utile `postcard`, suffixe CRC16-Modbus.
- **Identifiants de type de protocole** (`hare::codec`) — `ProtocolKind` et un conteneur
  opaque `ProtocolFrame` qui transporte une trame constructeur à travers la couche de
  liaison sans que hare ne l'analyse.
- **Hub de protocoles** (`hare::protocol`) — les briques protocolaires propres à la
  famille : CRC16 Modbus et extraction de registres/bobines, tramage requête/réponse du
  MC Protocol 3E de Mitsubishi, et le modèle zone/paramètre de S7comm avec rapports
  DB → SCL. Les protocoles qui exigent une pile cliente tierce complète restent dans le
  service de passerelle.
- **Traits HAL** (`hare::hal`) — les traits `Transport` et `SensorDevice` que les nœuds
  embassy implémentent pour leur propre matériel.
- **Analyseur ANSI/VT** (`hare::ansi`) — un analyseur de séquences d'échappement à machine
  à états qui émet des événements sémantiques, destiné aux terminaux et aux journaliseurs
  de protocole qui doivent comprendre un flux d'octets sans posséder de modèle d'écran.

## Installation

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false` est le bon choix pour un firmware embassy, mais les
fonctionnalités ne sont pas pleinement indépendantes : `wire` tire `hal`, et les traits HAL
s'appuient sur les types manifest et wire. Un firmware active normalement l'ensemble complet
(ou `wire,manifest`) plutôt qu'un module isolé.

### Drapeaux de fonctionnalités

| Fonctionnalité | Par défaut | Ce qu'elle active |
|---|---|---|
| `wire` | ✅ | Protocole de liaison Celestia : `Node` / `Gateway` / `Frame`, charges utiles postcard, CRC16 |
| `manifest` | ✅ | Schéma du manifeste matériel (`HardwareManifest`, cartes de registres, alarmes, échelles) |
| `codec` | ✅ | Identifiants de type de protocole et le conteneur opaque `ProtocolFrame` |
| `hal` | ✅ | Traits `Transport` / `SensorDevice` — implémentables côté embassy |
| `ansi` | ✅ | Analyseur de séquences d'échappement ANSI/VT (machine à états, no_std) |
| `std` | ❌ | Transmet `std` aux dépendances optionnelles `serde` / `postcard` pour les builds côté hôte |

`hare::protocol` est toujours compilé — il n'ajoute aucune dépendance.

Le manifeste est de simples données serde. hare **ne fournit pas** d'adaptateurs TOML ou
JSON : chargez-le avec le format serde de votre choix.

## Démarrage rapide

### Nœud capteur (firmware embassy)

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

`Node` est synchrone par conception : il boucle lui-même sur `Transport::recv`, ce qui le
rend compatible avec n'importe quel transport (UART embassy, émetteur-récepteur RS-485, un
pipe de test) et n'ajoute aucune dépendance d'exécution. La référence complète se trouve
dans [`examples/embassy_node.rs`](../../examples/embassy_node.rs) — un modèle qui compile
sur l'hôte comme bouchon et porte le câblage réel de la carte dans sa boucle principale
embassy commentée.

### Côté passerelle (evernight)

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

Un parcours complet limité à l'hôte (découverte, lecture de registre, télémétrie) figure
dans [`examples/host_demo.rs`](../../examples/host_demo.rs).

### Bout en bout sur un MCU émulé

Le membre de workspace [`qemu-mps2/`](../../qemu-mps2/) est une démonstration de firmware
Cortex-M4 qui s'exécute sous QEMU et remonte la télémétrie de température par UART ;
`qemu-mps2/examples/host_gateway.rs` décode ces trames sur l'hôte. Il sert également de
banc d'essai pour le protocole de liaison :

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

## Principes de conception

1. **`#![no_std]` + `alloc` uniquement** — ni std, ni tokio, ni appels système. La couche de
   liaison alloue ses tampons de trame via `alloc` ; prévoyez donc un tas sur la cible
   plutôt qu'une valeur figée.
2. **Serde partout** — les types de manifeste dérivent `Serialize`/`Deserialize` ; les
   charges utiles de liaison utilisent [`postcard`](https://docs.rs/postcard), un format
   binaire no_std compact.
3. **L'encodage est pur, les E/S appartiennent à l'appelant** — `Frame::encode` renvoie un
   `Vec<u8>` et `decode_frame` renvoie une `Frame` analysée ; l'appelant maîtrise le
   transport (UART embassy, port série tokio, IRQ du noyau) et la politique de réessai.
4. **Schéma partagé, implémentations indépendantes** — les nœuds embassy et evernight
   désérialisent tous deux le même `HardwareManifest`, mais leurs convertisseurs d'exécution
   restent séparés : un nœud no_std ne peut dépendre ni des types d'exécution ni des
   objets-traits de la passerelle.

## Relation avec le noyau KEI

Hare a été extrait du dépôt [`kei`](https://github.com/celestia-island/kei), où il vivait
sous `packages/kei/`. Le noyau est **gelé**
([feuille de route](https://docs.celestia.world)), et la crate-pont vit désormais ici afin
de pouvoir continuer d'évoluer alors que le noyau, lui, reste figé.

Les deux sont des artefacts indépendants qui partagent un contrat plutôt que du code :

| | Noyau KEI (`celestia-island/kei`) | Hare (cette crate) |
|---|---|---|
| Cible | **passerelles** ARM64 / RISC-V, ABI d'appels système Linux, MMU requise | **MCU** bare-metal, sans MMU |
| Chaîne d'outils | nightly, famille `aarch64-unknown-none` | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| Rôle | candidat à long terme pour remplacer Linux sur la passerelle | le contrat côté MCU |

Ce dépôt est autonome : Rust stable, son propre `rust-toolchain.toml`, et un workspace dont
l'unique membre (`qemu-mps2/`) vit dans ce répertoire. Le cloner suffit à obtenir un
workspace fonctionnel — aucun checkout du noyau et aucun override `rustup` nécessaires.

## Cibles et CI

| Contrôle | Cible |
|---|---|
| `test` | hôte `x86_64`, toutes les fonctionnalités |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, la cible embassy principale) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (cible secondaire) |
| `fmt` | hôte, `cargo fmt --all --check` |
| `clippy` | hôte, `--all-features --all-targets -- -D warnings` |
| `bench-check` | hôte, `cargo bench --no-run` |

Les contrôles CI s'exécutent sur les runners auto-hébergés de l'organisation ; le workflow
de publication (`.github/workflows/publish.yml`) s'exécute sur les runners hébergés par
GitHub, car ce dépôt est public. Les définitions des contrôles se trouvent dans
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

## Documentation

- Référence de l'API : [docs.rs/hare](https://docs.rs/hare)
- Traductions : [简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

Chaque `docs/<language>/README.md` est une traduction ; en cas de divergence, c'est le
README anglais qui fait foi.

## Licence

SySL-1.0 (Synthetic Source License) — voir [LICENSE](../../LICENSE).

---

## Provenance

Les modules source de la crate ont été extraits du dépôt du noyau
[`kei`](https://github.com/celestia-island/kei) (`packages/kei/`) le 2026-10-20 par décision
de l'utilisateur, renommés `kei-wire` à leur création puis `hare` (Omagari Hare, 小鉤晴) le
2026-10-21 par décision de l'utilisateur. Le noyau conserve son gel ; ce dépôt poursuit le
développement des crates-pont (wire / manifest / codec / hal / ansi). Le README, les
métadonnées du paquet et la documentation ont été réécrits lorsque la crate a été déplacée
vers son propre dépôt.
