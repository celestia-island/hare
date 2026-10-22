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
désérialisent le même manifeste matériel et parlent le même tramage de transport, si bien
qu'un firmware de capteur et un service de passerelle peuvent être développés
indépendamment et parvenir malgré tout à se rejoindre.

Il est `#![no_std]` + `alloc` : ni std, ni tokio, ni appels système, et aucun runtime
asynchrone propre — le nœud embassy fournit le transport, hare fournit le protocole.

## Fonctionnalités

- **Schéma de manifeste** (`hare::manifest`) — descriptions matérielles : cartes de
  registres, règles d'alarme, configurations de station et transformations d'échelle. Le
  même schéma que les deux côtés désérialisent.
- **Protocole de transport** (`hare::wire`) — un format de tramage binaire compact pour
  l'UART/USB-CDC/SPI entre les nœuds capteurs et la passerelle : magie `0xCE`, longueur
  `u16`, charge utile `postcard`, suffixe CRC16-Modbus.
- **Identifiants de type de protocole** (`hare::codec`) — `ProtocolKind` et un conteneur
  opaque `ProtocolFrame` pour Modbus, MC Protocol, EtherNet/IP-CIP, S7comm et CAN. La
  logique d'encodage/décodage relève d'evernight, pas de hare.
- **Traits HAL** (`hare::hal`) — les traits `Transport` et `SensorDevice` que les nœuds
  embassy implémentent sur leur propre matériel.
- **Analyseur ANSI/VT** (`hare::ansi`) — un analyseur de séquences d'échappement à machine
  à états qui émet des événements sémantiques, destiné aux terminaux et aux enregistreurs
  de protocole qui doivent comprendre un flux d'octets sans posséder de modèle d'écran.

## Installation

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

`default-features = false` est le bon choix pour un firmware embassy : les valeurs par
défaut constituent l'ensemble no_std complet (`wire`, `manifest`, `codec`, `hal`, `ansi`),
et un firmware n'a généralement besoin que des modules qu'il lie.

### Drapeaux de fonctionnalités

| Fonctionnalité | Par défaut | Ce qu'elle active |
|---|---|---|
| `wire` | ✅ | Protocole de transport Celestia : `Node` / `Gateway` / `Frame`, charges utiles postcard, CRC16 |
| `manifest` | ✅ | Schéma du manifeste matériel (`HardwareManifest`, cartes de registres, alarmes, échelles) |
| `codec` | ✅ | Identifiants de type de protocole et le conteneur opaque `ProtocolFrame` |
| `hal` | ✅ | Traits `Transport` / `SensorDevice` — implémentables côté embassy |
| `ansi` | ✅ | Analyseur de séquences d'échappement ANSI/VT (machine à états, no_std) |
| `std` | ❌ | Aides réservées à std (adaptateurs de manifeste TOML/JSON) pour le côté hôte |

## Démarrage rapide

### Nœud capteur (firmware embassy)

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

La référence complète est [`examples/embassy_node.rs`](./examples/embassy_node.rs) ; elle
ne compile pas sur l'hôte (il lui faut le HAL d'une carte) et est destinée à être copiée
dans un projet de firmware.

### Côté passerelle (evernight)

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

Une démonstration de bout en bout, limitée à l'hôte, se trouve dans
[`examples/host_demo.rs`](./examples/host_demo.rs).

### Bout en bout sur un MCU émulé

Le membre de workspace [`qemu-mps2/`](./qemu-mps2/) est une démonstration de firmware
Cortex-M4 qui s'exécute sous QEMU et remonte la télémétrie de température par UART ;
`examples/host_gateway.rs`, dans le même répertoire, décode ces trames sur l'hôte. Il sert
aussi de banc d'essai pour le protocole de transport :

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## Principes de conception

1. **`#![no_std]` + `alloc` uniquement** — ni std, ni tokio, ni appels système. La couche
   transport alloue ses tampons de trame via `alloc` ; prévoyez donc un tas sur la cible
   plutôt qu'une valeur fixe.
2. **Serde partout** — les types de manifeste dérivent `Serialize`/`Deserialize` ; les
   charges utiles du transport utilisent [`postcard`](https://docs.rs/postcard), un format
   binaire compact compatible no_std.
3. **Codec ≠ transport** — encodage et décodage sont des fonctions pures renvoyant
   `Vec<u8>` ; l'appelant maîtrise les E/S (embassy async, tokio, IRQ du noyau — tout
   fonctionne).
4. **Schéma partagé, implémentations indépendantes** — les nœuds embassy et evernight
   désérialisent le même `HardwareManifest`, mais leurs convertisseurs d'exécution restent
   séparés (no_std ne peut utiliser ni fermetures ni `Arc<dyn Fn>`).

## Relation avec le noyau KEI

Hare a été extrait du dépôt [`kei`](https://github.com/celestia-island/kei), où il vivait
sous `packages/kei/`. Le noyau est **gelé** ([feuille de
route](https://docs.celestia.world)), et la crate-pont vit désormais ici afin de pouvoir
continuer d'évoluer alors que le noyau, lui, reste figé.

Les deux sont des artefacts indépendants qui partagent un contrat plutôt que du code :

| | Noyau KEI (`celestia-island/kei`) | Hare (cette crate) |
|---|---|---|
| Cible | **passerelles** ARM64 / RISC-V, ABI d'appels système Linux, MMU requise | **MCU** bare-metal, pas de MMU |
| Chaîne d'outils | nightly, famille `aarch64-unknown-none` | stable, `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| Rôle | candidat à long terme pour remplacer Linux sur la passerelle | le contrat côté MCU |

Ce dépôt est autonome : Rust stable, son propre `rust-toolchain.toml`, et un workspace dont
l'unique membre (`qemu-mps2/`) vit dans ce répertoire. Le cloner suffit à obtenir un
workspace fonctionnel — aucun checkout du noyau ni override `rustup` n'est nécessaire.

## Cibles et CI

| Étape | Cible |
|---|---|
| `test` | hôte (`x86_64`), toutes les fonctionnalités |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M, la cible embassy principale) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (cible secondaire) |
| `fmt` / `clippy` / `bench-check` | hôte, `--all-targets` |

Tout s'exécute sur les runners auto-hébergés de l'organisation ; le workflow est
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

## Documentation

- Référence de l'API : [docs.rs/hare](https://docs.rs/hare)
- Traductions : [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

Ces pages sont des traductions du README anglais, fournies à titre de commodité ; en cas
de divergence, c'est la version anglaise qui fait foi.

## Licence

SySL-1.0 (Synthetic Source License) — voir [LICENSE](./LICENSE).

---

## Provenance

Extrait tel quel du dépôt du noyau [`kei`](https://github.com/celestia-island/kei)
(`packages/kei/`) le 2026-10-20 par décision de l'utilisateur, renommé `kei-wire` à sa
création puis `hare` (Omagari Hare, 小鉤晴) le 2026-10-21 par décision de l'utilisateur. Le
noyau conserve son gel ; ce dépôt poursuit le développement des crates-pont (wire /
manifest / codec / hal).
