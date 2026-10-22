<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/hare/master/docs/logo.webp" alt="hare" width="240" /></p>

<h1 align="center">Hare</h1>

<p align="center"><strong>مكتبة جسر مضمَّنة من نوع <code>#![no_std]</code> لمنظومة Celestia للإنترنت الأشياء</strong></p>

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
[Русский](../ru/README.md) ·
**العربية**

</div>

Hare هو طبقة العقد المشترك بين **عُقد الاستشعار القائمة على embassy** (وحدات تحكم
دقيقة تعمل على العتاد العاري) و**وسيط البوّابة** — المتمثّل اليوم في
[`evernight`](https://github.com/celestia-island/evernight) العامل على Linux. يُجري
الطرفان فكّ تسلسل لبيان العتاد نفسه، ويتحدّثان إطار النقل السلكي نفسه، بحيث يمكن بناء
برنامج ثابت (firmware) للاستشعار وخدمة بوّابة كلٌّ على حِدة، ومع ذلك يلتقيان في المنتصف.

وهي `#![no_std]` + `alloc`: بلا std، وبلا tokio، وبلا استدعاءات لنظام التشغيل، وبلا
زمن تشغيل غير متزامن خاص بها — فعقدة embassy هي التي توفّر النقل، و hare هو الذي يوفّر
البروتوكول.

## الميزات

- **مخطّط البيان** (`hare::manifest`) — أوصاف العتاد: خرائط السجلات، وقواعد الإنذار،
  وإعدادات المحطات، وتحويلات المقياس. وهو المخطّط ذاته الذي يفكّ الطرفان تسلسله.
- **بروتوكول النقل السلكي** (`hare::wire`) — بروتوكول تأطير ثنائي مُحكم الحجم لـ
  UART/USB-CDC/SPI بين عُقد الاستشعار والبوّابة: بايت سحري `0xCE`، وطول من نوع `u16`،
  وحمولة `postcard`، وذيل CRC16-Modbus.
- **معرّفات أنواع البروتوكولات** (`hare::codec`) — `ProtocolKind` وحاوية
  `ProtocolFrame` المعتمة من أجل Modbus و MC Protocol و EtherNet/IP-CIP و S7comm و CAN.
  أمّا منطق الترميز وفكّ الترميز نفسه فيعود إلى evernight لا إلى hare.
- **سمات HAL** (`hare::hal`) — سمتا `Transport` و `SensorDevice` اللتان تنفّذهما عُقد
  embassy وفق عتادها الخاص.
- **محلّل ANSI/VT** (`hare::ansi`) — محلّل تسلسلات هروب (escape sequences) قائم على آلة
  حالات، يُصدر أحداثًا دلالية، موجّه إلى الطرفيات ومسجّلات البروتوكولات التي تحتاج إلى
  فهم تدفّق البايتات دون أن تملك نموذج شاشة.

## التثبيت

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

يُعدّ `default-features = false` الخيار الصحيح لبرنامج ثابت قائم على embassy: فالقيم
الافتراضية هي مجموعة no_std الكاملة (`wire`، `manifest`، `codec`، `hal`، `ansi`)،
والبرنامج الثابت لا يحتاج عادةً إلا إلى الوحدات التي يربطها.

### أعلام الميزات

| الميزة | افتراضيًا | ما الذي تُتيحه |
|---|---|---|
| `wire` | ✅ | بروتوكول النقل السلكي الخاص بـ Celestia: `Node` / `Gateway` / `Frame`، وحمولات postcard، و CRC16 |
| `manifest` | ✅ | مخطّط بيان العتاد (`HardwareManifest`، وخرائط السجلات، والإنذارات، والمقاييس) |
| `codec` | ✅ | معرّفات أنواع البروتوكولات وحاوية `ProtocolFrame` المعتمة |
| `hal` | ✅ | سمتا `Transport` / `SensorDevice` — قابلتان للتنفيذ عبر embassy |
| `ansi` | ✅ | محلّل تسلسلات الهروب ANSI/VT (آلة حالات، no_std) |
| `std` | ❌ | أدوات مساعدة خاصة بـ std وحدها (مُحوّلات بيان TOML/JSON) للجهة المضيفة |

## البدء السريع

### عقدة الاستشعار (برنامج ثابت embassy)

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

المرجع الكامل هو [`examples/embassy_node.rs`](./examples/embassy_node.rs)؛ وهو لا
يُترجم على المضيف (فهو يحتاج إلى HAL للوحة)، والغرض منه أن يُنسخ إلى مشروع برنامج ثابت.

### جانب البوّابة (evernight)

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

يتوفّر عرض توضيحي كامل من طرف إلى طرف خاص بالمضيف في
[`examples/host_demo.rs`](./examples/host_demo.rs).

### من طرف إلى طرف على وحدة تحكم دقيقة مُحاكاة

عضو مساحة العمل [`qemu-mps2/`](./qemu-mps2/) هو عرض توضيحي لبرنامج ثابت على Cortex-M4
يعمل تحت QEMU ويُبلّغ عن قياسات الحرارة عن بُعد عبر UART؛ أمّا `examples/host_gateway.rs`
في الدليل نفسه فيفكّ تلك الإطارات على المضيف. وهو يخدم أيضًا كمنصّة اختبار لبروتوكول
النقل السلكي:

```bash
qemu-system-arm -M mps2-an386 -cpu cortex-m4 -m 16M \
    -display none -serial stdio \
    -kernel target/thumbv7em-none-eabi/release/kei-qemu-demo \
    | target/x86_64-unknown-linux-gnu/debug/examples/host_gateway
```

## مبادئ التصميم

1. **`#![no_std]` + `alloc` فقط** — بلا std، وبلا tokio، وبلا استدعاءات لنظام التشغيل.
   فطبقة النقل السلكي تُخصّص مخازن الإطارات عبر `alloc`، لذا احسب ميزانية كومة (heap) على
   الهدف بدلًا من رقم ثابت.
2. **Serde في كل مكان** — أنواع البيان تشتقّ `Serialize`/`Deserialize`؛ وحمولات النقل
   السلكي تستخدم [`postcard`](https://docs.rs/postcard)، وهي صيغة ثنائية مُحكمة الحجم تعمل
   بلا std.
3. **الترميز ≠ النقل** — الترميز وفكّ الترميز دوالّ خالصة تُعيد `Vec<u8>`؛ والمستدعي هو
   مالك الإدخال/الإخراج (غير المتزامن في embassy، أو tokio، أو مقاطعة النواة — وكلها
   تعمل).
4. **مخطّط مشترك وتطبيقات مستقلة** — تفكّ عُقد embassy و evernight تسلسل
   `HardwareManifest` نفسه، لكن محوّلات وقت التشغيل تبقى منفصلة بينهما (فبيئة لا تحتوي
   std لا تستطيع استخدام الإغلاقات أو `Arc<dyn Fn>`).

## العلاقة بنواة KEI

استُخرج Hare من مستودع [`kei`](https://github.com/celestia-island/kei) حيث كان يقيم تحت
`packages/kei/`. والنواة **مُجمَّدة** ([خارطة الطريق](https://docs.celestia.world))،
فيقيم صندوق الجسر الآن هنا ليواصل التطوّر بينما لا تتطوّر النواة.

والاثنان أثران مستقلان يتشاركان عقدًا لا شيفرة:

| | نواة KEI (`celestia-island/kei`) | Hare (هذا الصندوق) |
|---|---|---|
| الهدف | **بوّابات** ARM64 / RISC-V، وواجهة استدعاءات نظام Linux، وتتطلّب MMU | **وحدات تحكم دقيقة** على العتاد العاري، بلا MMU |
| سلسلة الأدوات | nightly، عائلة `aarch64-unknown-none` | stable، `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| الدور | مرشّح طويل الأمد لاستبدال Linux على البوّابة | العقد الخاص بجهة وحدة التحكم الدقيقة |

هذا المستودع مكتفٍ بذاته: Rust بإصدار stable، وملف `rust-toolchain.toml` خاص به،
ومساحة عمل عضوها الوحيد (`qemu-mps2/`) يقيم داخل هذا الدليل. واستنساخه يمنحك مساحة عمل
جاهزة للعمل — دون الحاجة إلى سحب نسخة من النواة ولا إلى تجاوز `rustup`.

## الأهداف و CI

| بوابة الفحص | الهدف |
|---|---|
| `test` | المضيف (`x86_64`)، مع جميع الميزات |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M، الهدف الأساسي لـ embassy) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (هدف ثانوي) |
| `fmt` / `clippy` / `bench-check` | المضيف، `--all-targets` |

يجري كل شيء على مُشغّلات المنظّمة المستضافة ذاتيًا؛ وملف سير العمل هو
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

## التوثيق

- مرجع الواجهة البرمجية: [docs.rs/hare](https://docs.rs/hare)
- الترجمات: [简体中文](./docs/zh-Hans/README.md) · [繁體中文](./docs/zh-Hant/README.md) ·
  [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) ·
  [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) ·
  [العربية](./docs/ar/README.md)

  هذه الصفحات ترجمات؛ ويبقى README بالإنجليزية هو المصدر الموثوق عند أي اختلاف.

## الترخيص

SySL-1.0 (Synthetic Source License) — راجع [LICENSE](./LICENSE).

---

## المنشأ

استُخرجت هذه المكتبة حرفيًا من مستودع نواة [`kei`](https://github.com/celestia-island/kei)
(`packages/kei/`) في 2026-10-20 بقرار من المستخدم، وسُمّيت `kei-wire` عند إنشائها، ثم
أُعيدت تسميتها إلى `hare` (Omagari Hare, 小鉤晴) في 2026-10-21 بقرار من المستخدم. وتحتفظ النواة
بتجميدها؛ بينما يواصل هذا المستودع حمل صناديق الجسر (wire / manifest / codec / hal) إلى
الأمام.
