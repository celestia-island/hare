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

Hare هو طبقة العقد المشتركة بين **عُقد الاستشعار القائمة على embassy** (وحدات تحكم دقيقة
تعمل على العتاد العاري) و**وسيط البوّابة** — المتمثّل اليوم في
[`evernight`](https://github.com/celestia-island/evernight) العامل على Linux. يفكّ الطرفان
تسلسل بيان العتاد نفسه، وينطقان تأطير النقل السلكي نفسه، بحيث يمكن بناء برنامج ثابت
للاستشعار وخدمة بوّابة كلٌّ على حِدة، ومع ذلك يلتقيان في المنتصف.

وهي `#![no_std]` + `alloc`: بلا std، وبلا tokio، وبلا استدعاءات لنظام التشغيل، وبلا زمن
تشغيل غير متزامن خاص بها — فعقدة embassy هي التي توفّر النقل، بينما يوفّر hare البروتوكول.

## الميزات

- **مخطّط البيان** (`hare::manifest`) — أوصاف العتاد: خرائط السجلات، وقواعد الإنذار،
  وإعدادات المحطات، وتحويلات القياس. وهو المخطّط نفسه الذي يفكّ الطرفان تسلسله.
- **بروتوكول النقل السلكي** (`hare::wire`) — بروتوكول تأطير ثنائي مُحكم الحجم من أجل
  UART/USB-CDC/SPI بين عُقد الاستشعار والبوّابة: بايت سحري `0xCE`، وطول من نوع `u16`،
  ونوع رسالة من نوع `u8`، وحمولة `postcard`، وذيل CRC16-Modbus.
- **معرّفات أنواع البروتوكولات** (`hare::codec`) — `ProtocolKind` وحاوية `ProtocolFrame`
  المعتمة التي تنقل إطارًا خاصًّا بمورّد عبر طبقة النقل السلكي دون أن يحلّله hare.
- **مركز البروتوكولات** (`hare::protocol`) — أصول البروتوكولات الخاصة بالعائلة: CRC16
  الخاص بـ Modbus واستخراج السجلات والملفات (registers/coils)، وتأطير الطلب/الاستجابة في
  Mitsubishi MC Protocol 3E، ونموذج المنطقة/المعامل (area/parameter) في S7comm مع تقارير
  DB → SCL. أمّا البروتوكولات التي تحتاج إلى حزمة عميل كاملة من طرف ثالث فتبقى في خدمة
  البوّابة.
- **سمات HAL** (`hare::hal`) — سمتا `Transport` و `SensorDevice` اللتان تنفّذهما عُقد
  embassy وفق عتادها الخاص.
- **محلّل ANSI/VT** (`hare::ansi`) — محلّل تسلسلات هروب قائم على آلة حالات، يُصدر أحداثًا
  دلالية، موجّه إلى الطرفيات ومسجّلات البروتوكولات التي تحتاج إلى فهم تدفّق البايتات دون
  أن تملك نموذج شاشة.

## التثبيت

```toml
[dependencies]
hare = { version = "0.1", default-features = false, features = ["wire", "manifest"] }
```

يُعدّ `default-features = false` الخيار الصحيح لبرنامج ثابت قائم على embassy، لكن الميزات
ليست مستقلة تمامًا: إذ يجذب `wire` الميزة `hal`، وسمات HAL مكتوبة اعتمادًا على أنواع
manifest وwire. لذلك يفعّل البرنامج الثابت المجموعة الكاملة عادةً (أو `wire,manifest`)
بدلًا من وحدة واحدة.

### أعلام الميزات

| الميزة | افتراضيًا | ما الذي تُتيحه |
|---|---|---|
| `wire` | ✅ | بروتوكول النقل السلكي الخاص بـ Celestia: `Node` / `Gateway` / `Frame`، وحمولات postcard، وCRC16 |
| `manifest` | ✅ | مخطّط بيان العتاد (`HardwareManifest`، وخرائط السجلات، والإنذارات، والمقاييس) |
| `codec` | ✅ | معرّفات أنواع البروتوكولات وحاوية `ProtocolFrame` المعتمة |
| `hal` | ✅ | سمتا `Transport` / `SensorDevice` — قابلتان للتنفيذ عبر embassy |
| `ansi` | ✅ | محلّل تسلسلات الهروب ANSI/VT (آلة حالات، no_std) |
| `std` | ❌ | يمرّر `std` إلى تبعيتَي `serde` / `postcard` الاختياريتين من أجل عمليات البناء على المضيف |

يُترجم `hare::protocol` دائمًا — ولا يضيف أي اعتماديات.

بيان العتاد مجرد بيانات serde عادية. لا يوفّر hare **أي** محوّلات TOML أو JSON: حمّله
بصيغة serde التي تختارها.

## البدء السريع

### عقدة الاستشعار (برنامج ثابت embassy)

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

صُمّم `Node` ليكون متزامنًا بحكم التصميم: فهو يدور بنفسه في حلقة على `Transport::recv`،
لذا يعمل مع أي ناقل (UART الخاص بـ embassy، أو مُرسِل/مُستقبِل RS-485، أو أنبوب اختبار)
ولا يضيف أي تبعية لزمن التشغيل. والمرجع الكامل موجود في
[`examples/embassy_node.rs`](../../examples/embassy_node.rs) — وهو قالب يُترجَم على المضيف
كبديل صوري (stub) ويحمل توصيلات اللوحة الفعلية داخل حلقة embassy الرئيسية المعلَّقة فيه.

### جانب البوّابة (evernight)

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

يتوفّر عرض تفصيلي كامل خاص بالمضيف وحده (الاكتشاف، وقراءة السجل، والقياس عن بُعد) في
[`examples/host_demo.rs`](../../examples/host_demo.rs).

### من طرف إلى طرف على وحدة تحكم دقيقة مُحاكاة

عضو مساحة العمل [`qemu-mps2/`](../../qemu-mps2/) هو عرض توضيحي لبرنامج ثابت على Cortex-M4
يعمل تحت QEMU ويُبلّغ عن قياسات الحرارة عن بُعد عبر UART؛ ويفكّ
`qemu-mps2/examples/host_gateway.rs` تلك الإطارات على المضيف. وهو يخدم أيضًا كمنصّة اختبار
لبروتوكول النقل السلكي:

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

## مبادئ التصميم

1. **`#![no_std]` + `alloc` فقط** — بلا std، وبلا tokio، وبلا استدعاءات لنظام التشغيل.
   فطبقة النقل السلكي تُخصّص مخازن الإطارات عبر `alloc`، لذا احسب ميزانية كومة (heap) على
   الهدف بدلًا من رقم ثابت.
2. **Serde في كل مكان** — أنواع البيان تشتقّ `Serialize`/`Deserialize`؛ وحمولات النقل
   السلكي تستخدم [`postcard`](https://docs.rs/postcard)، وهي صيغة ثنائية مُحكمة الحجم تعمل
   بلا std.
3. **الترميز نقيّ، والإدخال/الإخراج من مسؤولية المستدعي** — تُعيد `Frame::encode` قيمة من
   نوع `Vec<u8>`، وتُعيد `decode_frame` كائن `Frame` مُحلَّلًا؛ والمستدعي هو مالك الناقل
   (UART الخاص بـ embassy، أو المنفذ التسلسلي في tokio، أو مقاطعة النواة) وسياسة إعادة
   المحاولة.
4. **مخطّط مشترك وتطبيقات مستقلة** — يفكّ كلٌّ من عُقد embassy و evernight تسلسل
   `HardwareManifest` نفسه، لكن محوّلات وقت التشغيل تبقى منفصلة بينهما: فعقدة لا تحتوي
   std لا تستطيع الاعتماد على أنواع وقت التشغيل الخاصة بالبوّابة ولا على كائنات
   السمات (trait objects).

## العلاقة بنواة KEI

استُخرج Hare من مستودع [`kei`](https://github.com/celestia-island/kei) حيث كان يقيم تحت
`packages/kei/`. والنواة **مُجمَّدة** ([خارطة الطريق](https://docs.celestia.world))، فيقيم
صندوق الجسر الآن هنا ليواصل التطوّر بينما تتوقّف النواة عن ذلك.

والاثنان أثران مستقلان يتشاركان عقدًا لا شيفرة:

| | نواة KEI (`celestia-island/kei`) | Hare (هذا الصندوق) |
|---|---|---|
| الهدف | **بوّابات** ARM64 / RISC-V، وواجهة استدعاءات نظام Linux، وتتطلّب MMU | **وحدات تحكم دقيقة** على العتاد العاري، بلا MMU |
| سلسلة الأدوات | nightly، عائلة `aarch64-unknown-none` | stable، `thumbv7em-none-eabi` / `riscv32imc-unknown-none-elf` |
| الدور | مرشّح طويل الأمد لاستبدال Linux على البوّابة | العقد الخاص بجهة وحدة التحكم الدقيقة |

هذا المستودع مكتفٍ بذاته: Rust بإصدار stable، وملف `rust-toolchain.toml` خاص به، ومساحة
عمل عضوها الوحيد (`qemu-mps2/`) يقيم داخل هذا الدليل. واستنساخه يمنحك مساحة عمل جاهزة
للعمل — دون الحاجة إلى سحب نسخة من النواة ولا إلى تجاوز `rustup`.

## الأهداف و CI

| بوابة الفحص | الهدف |
|---|---|
| `test` | المضيف `x86_64`، مع جميع الميزات |
| `no_std-check` | `thumbv7em-none-eabi` (Cortex-M، الهدف الأساسي لـ embassy) |
| `no_std-riscv` | `riscv32imc-unknown-none-elf` (هدف ثانوي) |
| `fmt` | المضيف، `cargo fmt --all --check` |
| `clippy` | المضيف، `--all-features --all-targets -- -D warnings` |
| `bench-check` | المضيف، `cargo bench --no-run` |

تُشغَّل بوابات CI على مُشغّلات المنظّمة ذاتية الاستضافة؛ أمّا سير عمل الإصدار
(`.github/workflows/publish.yml`) فيعمل على مُشغّلات GitHub المستضافة لأن هذا المستودع
عام. وتُعرَّف البوابات في [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

## التوثيق

- مرجع الواجهة البرمجية: [docs.rs/hare](https://docs.rs/hare)
- الترجمات: [简体中文](../zh-Hans/README.md) · [繁體中文](../zh-Hant/README.md) ·
  [日本語](../ja/README.md) · [한국어](../ko/README.md) · [Français](../fr/README.md) ·
  [Español](../es/README.md) · [Русский](../ru/README.md) ·
  [العربية](../ar/README.md)

كل ملف `docs/<language>/README.md` هو ترجمة؛ ويبقى README بالإنجليزية هو المصدر الموثوق
عند أي اختلاف.

## الترخيص

SySL-1.0 (Synthetic Source License — رخصة المصدر التركيبي) — راجع [LICENSE](../../LICENSE).

---

## المنشأ

استُخرجت وحدات مصدر الصندوق من مستودع نواة
[`kei`](https://github.com/celestia-island/kei) (`packages/kei/`) في 2026-10-20 بقرار من
المستخدم، وسُمّيت `kei-wire` عند إنشائها، ثم أُعيدت تسميتها إلى `hare`
(Omagari Hare, 小鉤晴) في 2026-10-21 بقرار من المستخدم. وتحتفظ النواة بتجميدها؛ بينما
يواصل هذا المستودع حمل صناديق الجسر (wire / manifest / codec / hal / ansi) إلى الأمام.
وقد أُعيدت كتابة README وبيانات تعريف الحزمة والتوثيق عندما انتقل الصندوق إلى مستودعه
الخاص.
