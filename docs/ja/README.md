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

Hare は、**embassy ベースのセンサーノード**（ベアメタルの MCU）と**ゲートウェイブローカー**
（現在は Linux 上で動作する [`evernight`](https://github.com/celestia-island/evernight)）
との間にある共有契約レイヤーです。双方が同じハードウェアマニフェストをデシリアライズし、
同じワイヤフレーミングで通信するため、センサーファームウェアとゲートウェイサービスを
それぞれ独立に開発しても、両者はそのまま噛み合って動作します。

`#![no_std]` + `alloc` で動作します。std も tokio も OS コールもなく、独自の非同期
ランタイムも持ちません。トランスポートは embassy ノードが提供し、プロトコルは hare が
提供します。

## 特長

- **マニフェストスキーマ**（`hare::manifest`）— ハードウェアの記述：レジスタマップ、
  アラームルール、ステーション設定、スケール変換。双方がデシリアライズする共通の
  スキーマです。
- **ワイヤプロトコル**（`hare::wire`）— センサーノードとゲートウェイ間の
  UART/USB-CDC/SPI 向けのコンパクトなバイナリフレーミングプロトコル：`0xCE` マジック、
  `u16` 長、`u8` メッセージ種別、`postcard` ペイロード、CRC16-Modbus トレーラ。
- **プロトコル種別識別子**（`hare::codec`）— `ProtocolKind` と、ベンダー固有のフレームを
  hare が解析することなくワイヤ層経由で伝送する不透明な `ProtocolFrame` コンテナ。
- **プロトコルハブ**（`hare::protocol`）— ファミリー自身が持つプロトコル資産：Modbus の
  CRC16 とレジスタ／コイル抽出、三菱 MC Protocol 3E の要求／応答フレーミング、そして
  DB → SCL レポートを備えた S7comm のエリア／パラメータモデル。完全なサードパーティ製
  クライアントスタックを必要とするプロトコルは、ゲートウェイサービス側に留まります。
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

embassy ファームウェアでは `default-features = false` を選ぶのが適切ですが、各フィーチャーは
独立していません。`wire` は `hal` を引き込み、HAL トレイトは manifest と wire の型を前提に
書かれています。ファームウェアは通常、単一モジュールではなくフルセット（または
`wire,manifest`）を有効にします。

### フィーチャーフラグ

| フィーチャー | デフォルト | 有効になる内容 |
|---|---|---|
| `wire` | ✅ | Celestia ワイヤプロトコル：`Node` / `Gateway` / `Frame`、postcard ペイロード、CRC16 |
| `manifest` | ✅ | ハードウェアマニフェストスキーマ（`HardwareManifest`、レジスタマップ、アラーム、スケール） |
| `codec` | ✅ | プロトコル種別識別子と不透明な `ProtocolFrame` コンテナ |
| `hal` | ✅ | `Transport` / `SensorDevice` トレイト — embassy で実装可能 |
| `ansi` | ✅ | ANSI/VT エスケープシーケンスパーサ（ステートマシン、no_std） |
| `std` | ❌ | ホスト向けビルドで、オプションの `serde` / `postcard` 依存へ `std` を転送 |

`hare::protocol` は常にコンパイル対象で、依存関係は増えません。

マニフェストは単なる serde データです。hare は TOML / JSON アダプタを**同梱しません**。
任意の serde フォーマットで読み込んでください。

## クイックスタート

### センサーノード（embassy ファームウェア）

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

`Node` は設計上同期型です。自ら `Transport::recv` をループするため、どのような
トランスポート（embassy の UART、RS-485 トランシーバ、テスト用パイプ）でも動作し、
ランタイム依存も追加しません。完全なリファレンスは
[`examples/embassy_node.rs`](../../examples/embassy_node.rs) にあります。これはホスト上では
スタブとしてコンパイルされ、実際のボード配線はコメントアウトされた embassy のメインループに
記述されているテンプレートです。

### ゲートウェイ側（evernight）

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

ホストのみで完結する一連のウォークスルー（ディスカバリ、レジスタ読み出し、テレメトリ）は
[`examples/host_demo.rs`](../../examples/host_demo.rs) にあります。

### エミュレートされた MCU でのエンドツーエンド実行

ワークスペースメンバーの [`qemu-mps2/`](../../qemu-mps2/) は、QEMU 上で動作し UART 経由で
温度テレメトリを報告する Cortex-M4 ファームウェアのデモです。
`qemu-mps2/examples/host_gateway.rs` がそのフレームをホスト側でデコードします。これは
ワイヤプロトコルのテストハーネスも兼ねています：

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

## 設計原則

1. **`#![no_std]` + `alloc` のみ** — std も tokio も OS コールもありません。ワイヤ層は
   `alloc` を介してフレームバッファを確保するため、ターゲット上では固定値ではなくヒープ領域を
   見込んでください。
2. **全面的に Serde を採用** — マニフェスト型は `Serialize`/`Deserialize` を derive し、
   ワイヤペイロードにはコンパクトな no_std バイナリ形式である
   [`postcard`](https://docs.rs/postcard) を使用します。
3. **エンコードは純粋、I/O は呼び出し側に帰属** — `Frame::encode` は `Vec<u8>` を返し、
   `decode_frame` はパース済みの `Frame` を返します。トランスポート（embassy の UART、
   tokio のシリアル、カーネルの IRQ）とリトライポリシーは呼び出し側が所有します。
4. **スキーマは共有、実装は独立** — embassy ノードと evernight はどちらも同じ
   `HardwareManifest` をデシリアライズしますが、実行時の変換器は別々に保たれます。no_std の
   ノードは、ゲートウェイ側のランタイム型やトレイトオブジェクトに依存できません。

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

このリポジトリは自己完結しています。stable Rust、独自の `rust-toolchain.toml`、そして唯一の
メンバー（`qemu-mps2/`）がこのディレクトリ内にあるワークスペースで構成されています。
クローンすればそのまま動作するワークスペースが得られ、カーネルのチェックアウトも `rustup` の
オーバーライドも不要です。

## ターゲットと CI

| ゲート | ターゲット |
|---|---|
| `test` | ホスト `x86_64`、全フィーチャー |
| `no_std-check` | `thumbv7em-none-eabi`（Cortex-M、embassy の主要ターゲット） |
| `no_std-riscv` | `riscv32imc-unknown-none-elf`（副次ターゲット） |
| `fmt` | ホスト、`cargo fmt --all --check` |
| `clippy` | ホスト、`--all-features --all-targets -- -D warnings` |
| `bench-check` | ホスト、`cargo bench --no-run` |

CI ゲートは組織のセルフホストランナー上で実行されます。リリースワークフロー
（`.github/workflows/publish.yml`）は、このリポジトリが公開されているため GitHub ホストの
ランナー上で実行されます。ゲートの定義は
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) にあります。

## ドキュメント

- API リファレンス：[docs.rs/hare](https://docs.rs/hare)
- 翻訳：[简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

各 `docs/<language>/README.md` は翻訳です。翻訳と英語版の内容が食い違う場合は、英語版の
README が正となります。

## ライセンス

SySL-1.0（Synthetic Source License）。詳細は [LICENSE](../../LICENSE) を参照してください。

---

## 来歴

本クレートのソースモジュールは、2026-10-20 のユーザー裁定により
[`kei`](https://github.com/celestia-island/kei) カーネルリポジトリ（`packages/kei/`）から
抽出されました。作成時に `kei-wire` へ、さらに 2026-10-21 のユーザー裁定により `hare`
（Omagari Hare, 小鉤晴）へと改名されています。カーネルは凍結状態を維持し、このリポジトリが
ブリッジクレート（wire / manifest / codec / hal / ansi）を引き継いで発展させていきます。
クレートが独自リポジトリへ移った際に、README、パッケージメタデータ、ドキュメントが
書き直されました。
