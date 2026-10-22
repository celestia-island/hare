<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>Celestia IoT エコシステム向けの <code>#![no_std]</code> 組み込みブリッジライブラリ</strong></p>

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
**日本語** ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

Hare は、**embassy ベースのセンサーノード**（ベアメタルの MCU）と、**ゲートウェイブローカー**
（現在は Linux 上で動作する [`evernight`](https://github.com/celestia-island/evernight)）
との間にある共有契約レイヤーです。双方が同じハードウェアマニフェストをデシリアライズし、
同じワイヤフレーミングで通信するため、センサーファームウェアとゲートウェイサービスは
それぞれ独立して開発しても、そのまま組み合わせて動作させることができます。

`#![no_std]` + `alloc` で動作します。std も tokio も OS コールもなく、独自の非同期
ランタイムも持ちません。トランスポートは embassy ノードが提供し、プロトコルは hare が
提供します。

## 特長

- **マニフェストスキーマ**（`hare::manifest`）— ハードウェアの記述：レジスタマップ、
  アラームルール、ステーション設定、スケール変換。双方がデシリアライズする共通の
  スキーマです。
- **ワイヤプロトコル**（`hare::wire`）— センサーノードとゲートウェイ間の
  UART/USB-CDC/SPI 向けのコンパクトなバイナリフレーミングプロトコル：`0xCE` マジック、
  `u16` 長、`postcard` ペイロード、CRC16-Modbus トレーラ。
- **プロトコル種別識別子**（`hare::codec`）— `ProtocolKind` と、Modbus、MC プロトコル、
  EtherNet/IP-CIP、S7comm、CAN 用の不透明な `ProtocolFrame` コンテナ。エンコード／
  デコードのロジック自体は hare ではなく evernight に属します。
- **HAL トレイト**（`hare::hal`）— embassy ノードが自前のハードウェアに対して実装する
  `Transport` および `SensorDevice` トレイト。
- **ANSI/VT パーサ**（`hare::ansi`）— 意味的イベントを出力するステートマシン方式の
  エスケープシーケンスパーサ。画面モデルを持たずにバイトストリームを解釈する必要がある
  ターミナルやプロトコルロガー向けです。

## インストール

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

embassy ファームウェアでは `default-features = false` を選ぶのが適切です。デフォルトは
no_std 向けのフルセット（`wire`、`manifest`、`codec`、`hal`、`ansi`）であり、ファームウェアは
通常、自身がリンクするモジュールだけを必要とします。

### フィーチャーフラグ

| フィーチャー | デフォルト | 有効になる内容 |
|---|---|---|
| `wire` | ✅ | Celestia ワイヤプロトコル：`Node` / `Gateway` / `Frame`、postcard ペイロード、CRC16 |
| `manifest` | ✅ | ハードウェアマニフェストスキーマ（`HardwareManifest`、レジスタマップ、アラーム、スケール） |
| `codec` | ✅ | プロトコル種別識別子と不透明な `ProtocolFrame` コンテナ |
| `hal` | ✅ | `Transport` / `SensorDevice` トレイト — embassy で実装可能 |
| `ansi` | ✅ | ANSI/VT エスケープシーケンスパーサ（ステートマシン、no_std） |
| `std` | ❌ | ホスト側向けの std 専用ヘルパー（TOML/JSON マニフェストアダプタ） |

## クイックスタート

### センサーノード（embassy ファームウェア）

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

完全なリファレンスは [`examples/embassy_node.rs`](./examples/embassy_node.rs) にあります。
これはホスト上ではコンパイルされず（ボード固有の HAL が必要）、ファームウェア
プロジェクトにコピーして使うことを想定しています。

### ゲートウェイ側（evernight）

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

ホスト専用のエンドツーエンドデモは [`examples/host_demo.rs`](./examples/host_demo.rs) に
あります。

### エミュレートされた MCU でのエンドツーエンド実行

ワークスペースメンバー [`qemu-mps2/`](./qemu-mps2/) は、QEMU 上で動作し UART 経由で
温度テレメトリを報告する Cortex-M4 ファームウェアのデモです。同じディレクトリ内の
`examples/host_gateway.rs` がそのフレームをホスト側でデコードします。これはワイヤ
プロトコルのテストハーネスも兼ねています：

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## 設計原則

1. **`#![no_std]` + `alloc` のみ** — std も tokio も OS コールもありません。ワイヤ層は
   `alloc` を介してフレームバッファを確保するため、ターゲット上では固定サイズではなく
   ヒープ領域を見込んでください。
2. **全面的に Serde を採用** — マニフェスト型は `Serialize`/`Deserialize` を derive し、
   ワイヤペイロードには [`postcard`](https://docs.rs/postcard) というコンパクトな
   no_std バイナリ形式を使用します。
3. **コーデック ≠ トランスポート** — エンコード／デコードは `Vec<u8>` を返す純粋関数で
   あり、I/O は呼び出し側が所有します（embassy の非同期、tokio、カーネル IRQ のいずれでも
   動作します）。
4. **スキーマは共有、実装は独立** — embassy ノードと evernight はどちらも同じ
   `HardwareManifest` をデシリアライズしますが、実行時の変換器は別々に保たれます
   （no_std ではクロージャや `Arc<dyn Fn>` を使用できません）。

## KEI カーネルとの関係

Hare は [`kei`](https://github.com/celestia-island/kei) リポジトリの `packages/kei/` から
抽出されました。カーネルは**凍結**されており（[ロードマップ](https://docs.celestia.world)）、
ブリッジクレートはカーネルが凍結されたままでも進化を続けられるよう、現在はこのリポジトリに
置かれています。

両者はコードではなく契約を共有する、独立した成果物です：

| | KEI カーネル（`celestia-island/kei`） | Hare（本クレート） |
|---|---|---|
| 対象 | ARM64 / RISC-V の**ゲートウェイ**、Linux システムコール ABI、MMU 必須 | ベアメタルの **MCU**、MMU なし |
| ツールチェーン | nightly、`aarch64-unknown-none` ファミリ | stable、`thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| 役割 | ゲートウェイ上の Linux を置き換える長期的な候補 | MCU 側の契約 |

このリポジトリは自己完結しています。stable Rust、独自の `rust-toolchain.toml`、そして
唯一のメンバー（`qemu-mps2/`）がこのディレクトリ内にあるワークスペースで構成されています。
クローンすればそのまま動作するワークスペースが得られ、カーネルのチェックアウトも
`rustup` のオーバーライドも不要です。

## ターゲットと CI

| ゲート | ターゲット |
|---|---|
| `test` | ホスト（`x86_64`）、全フィーチャー |
| `no_std-check` | `thumbv7em-none-eabi`（Cortex-M、embassy の主要ターゲット） |
| `no_std-riscv` | `riscv32imc-unknown-none-elf`（副次ターゲット） |
| `fmt` / `clippy` / `bench-check` | ホスト、`--all-targets` |

すべて組織のセルフホストランナー上で実行されます。ワークフローは
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml) にあります。

## ドキュメント

- API リファレンス：[docs.rs/hare](https://docs.rs/hare)
- 翻訳：[简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

本ページは各言語への翻訳ドキュメントのひとつです。記述の正は英語版 README であり、
翻訳との間に齟齬がある場合は英語版を参照してください。

## ライセンス

SySL-1.0（Synthetic Source License）に基づいて提供されます。詳細は [LICENSE](./LICENSE) を
参照してください。

---

## 来歴

2026-10-20 のユーザー裁定により [`kei`](https://github.com/celestia-island/kei) カーネル
リポジトリ（`packages/kei/`）からそのまま抽出され、作成時に `kei-wire` へ、2026-10-21 の
ユーザー裁定により `hare`（Omagari Hare, 小鉤晴）へと改名されました。カーネルは凍結状態を
維持し、このリポジトリがブリッジクレート（wire / manifest / codec / hal）を引き継いで
発展させていきます。
