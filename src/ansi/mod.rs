//! ANSI/VT escape sequence parser (hare's `ansi` feature).
//!
//! A **state-machine** parser after the Williams/Windows-Terminal model:
//! bytes in, semantic events out. The parser owns NO screen state —
//! consumers (a terminal grid, a protocol logger, a test harness) drive
//! their own model from the emitted [`AnsiEvent`]s. This separation is
//! what lets the same parser serve a web terminal renderer (via the
//! wire), a bare-metal diagnostic console, and a headless pty logger.
//!
//! ## Coverage (v1)
//!
//! - C0 controls (CR, LF, BS, TAB, BEL, …)
//! - CSI sequences (`ESC [ params letter`) with SGR attribute parsing
//! - OSC strings (`ESC ] ... BEL / ST`) — title, hyperlink (OSC 8), etc.
//! - DEC private modes (`ESC [ ? n h/l`)
//! - Cursor/erase ops (CUU/CUD/CUF/CUB, ED, EL)
//! - Scroll region (DECSTBM)
//! - Character-set selection (ESC ( B etc.) — acknowledged, no-op
//!
//! ## NOT yet covered (registered as follow-ups)
//!
//! - DCS/APC/PM string payloads (SIXEL, kitty graphics ride APC — the
//!   web terminal's image addons parse these on their side today)
//! - Device status reports (DSR) — needs a response channel
//! - Wide/combining character width tables (the grid consumer owns
//!   width policy; the parser passes code points through verbatim)

use alloc::string::String;
use alloc::vec::Vec;

// ── Events ─────────────────────────────────────────────────────────────

/// One parsed semantic unit from the byte stream.
#[derive(Debug, Clone, PartialEq)]
pub enum AnsiEvent {
    /// A printable character (or a control the consumer should render
    /// verbatim). UTF-8 is assembled here — partial sequences buffer
    /// until complete.
    Char(char),
    /// Carriage return.
    Cr,
    /// Line feed (also covers NL/VT/FF — terminals treat them alike).
    Lf,
    /// Backspace.
    Bs,
    /// Horizontal tab.
    Tab,
    /// Bell.
    Bell,
    /// A CSI sequence completed. `params` are the semicolon-split
    /// integers (empty ⇒ [0]); `intermediates` are the `!`, `"`, …
    /// bytes before the final; `final` is the terminating letter.
    /// `private` carries the `?` (DEC) or `>` flag when present.
    Csi {
        private: Option<u8>,
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// An OSC string completed (the terminator was BEL or ST).
    /// `code` is the leading integer; `payload` is the rest verbatim.
    Osc { code: u16, payload: String },
    /// A mode set/reset (`ESC [ ? n h` / `l`). `set` = h.
    ModeSet { mode: u16, set: bool },
    /// Character-set selection — acknowledged, consumers no-op it.
    CharsetSelect { which: u8, target: u8 },
    /// An unknown/simple escape (`ESC x`).
    Esc { final_byte: u8 },
    /// The UTF-8 assembler hit an invalid sequence — the replacement
    /// character was substituted.
    InvalidUtf8,
}

// ── SGR (Select Graphic Rendition) ────────────────────────────────────

/// One SGR attribute change, parsed from `ESC [ ... m` params.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SgrAttr {
    Reset,
    Bold(bool),
    Dim(bool),
    Italic(bool),
    Underline(bool),
    Blink(bool),
    Inverse(bool),
    Hidden(bool),
    Strikethrough(bool),
    /// Foreground: the standard 0-7 / 8-15 / 256-color / 24-bit set.
    Fg(Color),
    /// Background.
    Bg(Color),
    /// An unrecognized SGR code — passed through for forward compat.
    Other(u16),
}

/// A parsed color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Reset to the default (SGR 39 / 49).
    Default,
    /// Standard palette index (0-7 normal, 8-15 bright).
    Indexed(u8),
    /// 256-color palette index.
    Palette(u8),
    /// 24-bit RGB (SGR 38;2;r;g;b).
    Rgb(u8, u8, u8),
}

/// Parse a completed SGR sequence's params into attribute changes.
/// The empty param list (`ESC [ m`) is SGR 0 (reset) — normalized here.
pub fn parse_sgr(params: &[u16]) -> Vec<SgrAttr> {
    let mut out = Vec::new();
    if params.is_empty() || (params.len() == 1 && params[0] == 0) {
        out.push(SgrAttr::Reset);
        return out;
    }
    let mut i = 0;
    while i < params.len() {
        let p = params[i];
        match p {
            0 => out.push(SgrAttr::Reset),
            1 => out.push(SgrAttr::Bold(true)),
            2 => out.push(SgrAttr::Dim(true)),
            3 => out.push(SgrAttr::Italic(true)),
            4 => out.push(SgrAttr::Underline(true)),
            5 => out.push(SgrAttr::Blink(true)),
            7 => out.push(SgrAttr::Inverse(true)),
            8 => out.push(SgrAttr::Hidden(true)),
            9 => out.push(SgrAttr::Strikethrough(true)),
            21 => out.push(SgrAttr::Bold(false)),
            22 => out.push(SgrAttr::Dim(false)),
            23 => out.push(SgrAttr::Italic(false)),
            24 => out.push(SgrAttr::Underline(false)),
            25 => out.push(SgrAttr::Blink(false)),
            27 => out.push(SgrAttr::Inverse(false)),
            28 => out.push(SgrAttr::Hidden(false)),
            29 => out.push(SgrAttr::Strikethrough(false)),
            30..=37 => out.push(SgrAttr::Fg(Color::Indexed(p as u8 - 30))),
            38 => {
                // Extended foreground: 38;5;n or 38;2;r;g;b
                if i + 1 < params.len() {
                    match params[i + 1] {
                        5 if i + 2 < params.len() => {
                            out.push(SgrAttr::Fg(Color::Palette(params[i + 2] as u8)));
                            i += 2;
                        }
                        2 if i + 4 < params.len() => {
                            out.push(SgrAttr::Fg(Color::Rgb(
                                params[i + 2] as u8,
                                params[i + 3] as u8,
                                params[i + 4] as u8,
                            )));
                            i += 4;
                        }
                        _ => {}
                    }
                }
            }
            39 => out.push(SgrAttr::Fg(Color::Default)),
            40..=47 => out.push(SgrAttr::Bg(Color::Indexed(p as u8 - 40))),
            48 => {
                if i + 1 < params.len() {
                    match params[i + 1] {
                        5 if i + 2 < params.len() => {
                            out.push(SgrAttr::Bg(Color::Palette(params[i + 2] as u8)));
                            i += 2;
                        }
                        2 if i + 4 < params.len() => {
                            out.push(SgrAttr::Bg(Color::Rgb(
                                params[i + 2] as u8,
                                params[i + 3] as u8,
                                params[i + 4] as u8,
                            )));
                            i += 4;
                        }
                        _ => {}
                    }
                }
            }
            49 => out.push(SgrAttr::Bg(Color::Default)),
            90..=97 => out.push(SgrAttr::Fg(Color::Indexed(p as u8 - 90 + 8))),
            100..=107 => out.push(SgrAttr::Bg(Color::Indexed(p as u8 - 100 + 8))),
            _ => out.push(SgrAttr::Other(p)),
        }
        i += 1;
    }
    out
}

// ── Parser state machine ──────────────────────────────────────────────

/// The parser's discrete states (the VT parser's canonical diagram).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Escape,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    OscString,
    /// `ESC (`, `ESC )`, `ESC #`, … — expecting the charset byte.
    Charset,
    /// A state we skip until a known terminator (DCS/APC/PM).
    StringIgnore,
}

/// Incremental ANSI/VT parser: feed bytes, drain events.
///
/// ```
/// use hare::ansi::AnsiParser;
/// let mut p = AnsiParser::new();
/// p.feed(b"hello\x1b[31mworld");
/// let events = p.drain();
/// // 10 Char events + 1 Csi (SGR red) + 5 Char events
/// assert_eq!(events.len(), 16);
/// ```
pub struct AnsiParser {
    state: State,
    /// CSI accumulation: private flag, params, intermediates.
    csi_private: Option<u8>,
    csi_params: Vec<u16>,
    csi_current_param: Option<u16>,
    csi_intermediates: Vec<u8>,
    /// OSC accumulation: the code + payload string.
    osc_code: u16,
    osc_payload: String,
    /// Charset selector (`(`, `)`, …).
    charset_which: u8,
    /// UTF-8 assembler buffer.
    utf8_buf: [u8; 4],
    utf8_len: usize,
    utf8_need: usize,
    /// Whether the OSC payload has passed its semicolon (code|payload).
    osc_seen_semi: bool,
    /// The event output buffer.
    events: Vec<AnsiEvent>,
}

impl Default for AnsiParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiParser {
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            csi_private: None,
            csi_params: Vec::new(),
            csi_current_param: None,
            csi_intermediates: Vec::new(),
            osc_code: 0,
            osc_payload: String::new(),
            charset_which: 0,
            utf8_buf: [0; 4],
            utf8_len: 0,
            utf8_need: 0,
            osc_seen_semi: false,
            events: Vec::new(),
        }
    }

    /// Feed raw bytes. Events buffer internally; call [`drain`].
    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.step(b);
        }
    }

    /// Drain all buffered events.
    pub fn drain(&mut self) -> Vec<AnsiEvent> {
        core::mem::take(&mut self.events)
    }

    // Internal: the state machine step.
    fn step(&mut self, b: u8) {
        match self.state {
            State::Ground => self.step_ground(b),
            State::Escape => self.step_escape(b),
            State::CsiEntry => self.step_csi_entry(b),
            State::CsiParam => self.step_csi_param(b),
            State::CsiIntermediate => self.step_csi_intermediate(b),
            State::CsiIgnore => self.step_csi_ignore(b),
            State::OscString => self.step_osc(b),
            State::Charset => {
                self.push(AnsiEvent::CharsetSelect {
                    which: self.charset_which,
                    target: b,
                });
                self.state = State::Ground;
            }
            State::StringIgnore => {
                // DCS/APC/PM: skip until ST (ESC \) or BEL.
                if b == 0x07 {
                    self.state = State::Ground;
                } else if b == 0x1b {
                    // Might be ST (ESC \); the next byte decides — for
                    // simplicity, treat ESC in a string as the string's
                    // end (true ST is the only legal ESC in a string).
                    self.state = State::Escape;
                }
            }
        }
    }

    fn step_ground(&mut self, b: u8) {
        match b {
            0x1b => {
                self.flush_utf8();
                self.state = State::Escape;
            }
            b'\r' => {
                self.flush_utf8();
                self.push(AnsiEvent::Cr);
            }
            b'\n' | 0x0b | 0x0c => {
                self.flush_utf8();
                self.push(AnsiEvent::Lf);
            }
            0x08 => {
                self.flush_utf8();
                self.push(AnsiEvent::Bs);
            }
            b'\t' => {
                self.flush_utf8();
                self.push(AnsiEvent::Tab);
            }
            0x07 => {
                self.flush_utf8();
                self.push(AnsiEvent::Bell);
            }
            0x00..=0x1f | 0x7f => {
                // Other C0 controls: pass through as-is (the consumer
                // decides; most are no-ops in a grid).
            }
            _ => self.feed_utf8(b),
        }
    }

    fn step_escape(&mut self, b: u8) {
        match b {
            b'[' => {
                self.state = State::CsiEntry;
                self.csi_private = None;
                self.csi_params.clear();
                self.csi_current_param = None;
                self.csi_intermediates.clear();
            }
            b']' => {
                self.state = State::OscString;
                self.osc_code = 0;
                self.osc_payload.clear();
                self.osc_seen_semi = false;
            }
            b'P' | b'X' | b'^' | b'_' => {
                // DCS / SOS / PM / APC — string payload we skip (v1).
                self.state = State::StringIgnore;
            }
            b'(' | b')' | b'*' | b'+' | b'#' | b'%' => {
                self.charset_which = b;
                self.state = State::Charset;
            }
            0x1b => {
                // Double ESC — treat as unknown esc then re-enter.
                self.push(AnsiEvent::Esc { final_byte: 0x1b });
                self.state = State::Escape;
            }
            _ => {
                self.push(AnsiEvent::Esc { final_byte: b });
                self.state = State::Ground;
            }
        }
    }

    fn step_csi_entry(&mut self, b: u8) {
        match b {
            b'?' | b'>' | b'<' | b'=' => {
                self.csi_private = Some(b);
                self.state = State::CsiParam;
            }
            b'0'..=b'9' => {
                self.csi_current_param = Some((b - b'0') as u16);
                self.state = State::CsiParam;
            }
            b';' => {
                self.csi_params.push(0);
                self.state = State::CsiParam;
            }
            b':' | b'!' | b'"' | b' '..=b'/' if b != b' ' => {
                self.csi_intermediates.push(b);
                self.state = State::CsiIntermediate;
            }
            0x40..=0x7e => {
                self.finish_csi(b);
            }
            _ => {
                self.state = State::CsiIgnore;
            }
        }
    }

    fn step_csi_param(&mut self, b: u8) {
        match b {
            b'0'..=b'9' => {
                let d = (b - b'0') as u16;
                self.csi_current_param = Some(
                    self.csi_current_param
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(d),
                );
            }
            b';' => {
                self.csi_params
                    .push(self.csi_current_param.take().unwrap_or(0));
            }
            b':' => {
                // Sub-parameter separator — treat like `;` (v1).
                self.csi_params
                    .push(self.csi_current_param.take().unwrap_or(0));
            }
            b'!'..=b'/' | b' ' => {
                self.csi_params
                    .push(self.csi_current_param.take().unwrap_or(0));
                self.csi_intermediates.push(b);
                self.state = State::CsiIntermediate;
            }
            0x40..=0x7e => {
                self.finish_csi(b);
            }
            _ => {
                self.state = State::CsiIgnore;
            }
        }
    }

    fn step_csi_intermediate(&mut self, b: u8) {
        if (0x20..=0x2f).contains(&b) {
            self.csi_intermediates.push(b);
        } else if (0x40..=0x7e).contains(&b) {
            self.finish_csi(b);
        } else {
            self.state = State::CsiIgnore;
        }
    }

    fn step_csi_ignore(&mut self, b: u8) {
        if (0x40..=0x7e).contains(&b) {
            self.state = State::Ground;
        }
    }

    fn finish_csi(&mut self, final_byte: u8) {
        self.state = State::Ground;
        if self.csi_current_param.is_some() || self.csi_params.is_empty() {
            self.csi_params
                .push(self.csi_current_param.take().unwrap_or(0));
        }
        let private = self.csi_private.take();
        let params = core::mem::take(&mut self.csi_params);
        let intermediates = core::mem::take(&mut self.csi_intermediates);
        // DEC private mode set/reset: emit as ModeSet for convenience.
        if private == Some(b'?') && (final_byte == b'h' || final_byte == b'l') {
            let set = final_byte == b'h';
            for &mode in &params {
                self.push(AnsiEvent::ModeSet { mode, set });
            }
            return;
        }
        self.push(AnsiEvent::Csi {
            private,
            params,
            intermediates,
            final_byte,
        });
    }

    // ── OSC ────────────────────────────────────────────────────────────

    fn step_osc(&mut self, b: u8) {
        match b {
            0x07 => {
                // BEL terminator.
                self.finish_osc();
            }
            0x1b => {
                // Might be ST (ESC \) — the simple path treats the ESC
                // as the terminator (true ST has \ as the next byte,
                // which would be an Esc-event with final \, harmless).
                self.finish_osc();
                self.state = State::Escape;
            }
            _ => {
                if !self.osc_seen_semi {
                    if b.is_ascii_digit() {
                        self.osc_code = self.osc_code.saturating_mul(10) + (b - b'0') as u16;
                    } else if b == b';' {
                        self.osc_seen_semi = true;
                    }
                    // Anything else before the semicolon: ignore (be
                    // permissive — OSC codes are numeric).
                } else {
                    self.osc_payload.push(b as char);
                }
            }
        }
    }

    fn finish_osc(&mut self) {
        self.state = State::Ground;
        let code = self.osc_code;
        let payload = core::mem::take(&mut self.osc_payload);
        self.push(AnsiEvent::Osc { code, payload });
        self.osc_seen_semi = false;
    }

    // ── UTF-8 assembler ───────────────────────────────────────────────

    fn feed_utf8(&mut self, b: u8) {
        if self.utf8_need == 0 {
            if b < 0x80 {
                self.push(AnsiEvent::Char(b as char));
            } else if b & 0xe0 == 0xc0 {
                self.utf8_need = 1;
                self.utf8_buf[0] = b;
                self.utf8_len = 1;
            } else if b & 0xf0 == 0xe0 {
                self.utf8_need = 2;
                self.utf8_buf[0] = b;
                self.utf8_len = 1;
            } else if b & 0xf8 == 0xf0 {
                self.utf8_need = 3;
                self.utf8_buf[0] = b;
                self.utf8_len = 1;
            } else {
                self.push(AnsiEvent::InvalidUtf8);
                self.push(AnsiEvent::Char(char::REPLACEMENT_CHARACTER));
            }
        } else {
            if b & 0xc0 == 0x80 {
                self.utf8_buf[self.utf8_len] = b;
                self.utf8_len += 1;
                if self.utf8_len == self.utf8_need + 1 {
                    let s = core::str::from_utf8(&self.utf8_buf[..self.utf8_len]);
                    match s.ok().and_then(|s| s.chars().next()) {
                        Some(c) => self.push(AnsiEvent::Char(c)),
                        None => {
                            self.push(AnsiEvent::InvalidUtf8);
                            self.push(AnsiEvent::Char(char::REPLACEMENT_CHARACTER));
                        }
                    }
                    self.utf8_need = 0;
                    self.utf8_len = 0;
                }
            } else {
                // Invalid continuation — flush what we had, reprocess.
                self.push(AnsiEvent::InvalidUtf8);
                self.push(AnsiEvent::Char(char::REPLACEMENT_CHARACTER));
                self.utf8_need = 0;
                self.utf8_len = 0;
                self.feed_utf8(b);
            }
        }
    }

    fn flush_utf8(&mut self) {
        if self.utf8_need > 0 {
            self.push(AnsiEvent::InvalidUtf8);
            self.push(AnsiEvent::Char(char::REPLACEMENT_CHARACTER));
            self.utf8_need = 0;
            self.utf8_len = 0;
        }
    }

    fn push(&mut self, event: AnsiEvent) {
        self.events.push(event);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn events(input: &[u8]) -> Vec<AnsiEvent> {
        let mut p = AnsiParser::new();
        p.feed(input);
        p.drain()
    }

    #[test]
    fn plain_text_passes_through_as_chars() {
        let ev = events(b"hello");
        assert_eq!(ev.len(), 5);
        assert!(ev
            .iter()
            .all(|e| matches!(e, AnsiEvent::Char(c) if "hello".contains(*c))));
    }

    #[test]
    fn controls_map_to_their_events() {
        let ev = events(b"a\r\nb\tc\x07d\x08e");
        assert_eq!(ev[0], AnsiEvent::Char('a'));
        assert_eq!(ev[1], AnsiEvent::Cr);
        assert_eq!(ev[2], AnsiEvent::Lf);
        assert_eq!(ev[3], AnsiEvent::Char('b'));
        assert_eq!(ev[4], AnsiEvent::Tab);
        assert_eq!(ev[5], AnsiEvent::Char('c'));
        assert_eq!(ev[6], AnsiEvent::Bell);
        assert_eq!(ev[7], AnsiEvent::Char('d'));
        assert_eq!(ev[8], AnsiEvent::Bs);
        assert_eq!(ev[9], AnsiEvent::Char('e'));
    }

    #[test]
    fn csi_cursor_up_parses_params() {
        let ev = events(b"\x1b[5A");
        assert_eq!(ev.len(), 1);
        match &ev[0] {
            AnsiEvent::Csi {
                params, final_byte, ..
            } => {
                assert_eq!(*params, vec![5]);
                assert_eq!(*final_byte, b'A');
            }
            other => panic!("expected Csi, got {other:?}"),
        }
    }

    #[test]
    fn csi_empty_params_normalize_to_zero() {
        let ev = events(b"\x1b[m");
        match &ev[0] {
            AnsiEvent::Csi {
                params, final_byte, ..
            } => {
                assert_eq!(*params, vec![0]);
                assert_eq!(*final_byte, b'm');
            }
            other => panic!("expected Csi, got {other:?}"),
        }
    }

    #[test]
    fn dec_private_mode_emits_mode_set() {
        let ev = events(b"\x1b[?25h");
        assert_eq!(ev.len(), 1);
        assert_eq!(
            ev[0],
            AnsiEvent::ModeSet {
                mode: 25,
                set: true
            }
        );
        let ev = events(b"\x1b[?25l");
        assert_eq!(
            ev[0],
            AnsiEvent::ModeSet {
                mode: 25,
                set: false
            }
        );
    }

    #[test]
    fn osc_title_parses_code_and_payload() {
        let ev = events(b"\x1b]0;My Terminal\x07");
        assert_eq!(ev.len(), 1);
        match &ev[0] {
            AnsiEvent::Osc { code, payload } => {
                assert_eq!(*code, 0);
                assert_eq!(payload, "My Terminal");
            }
            other => panic!("expected Osc, got {other:?}"),
        }
    }

    #[test]
    fn sgr_red_foreground() {
        let ev = events(b"\x1b[31m");
        match &ev[0] {
            AnsiEvent::Csi {
                params, final_byte, ..
            } => {
                assert_eq!(*final_byte, b'm');
                let attrs = parse_sgr(params);
                assert_eq!(attrs, vec![SgrAttr::Fg(Color::Indexed(1))]);
            }
            other => panic!("expected Csi, got {other:?}"),
        }
    }

    #[test]
    fn sgr_24bit_color() {
        let attrs = parse_sgr(&[38, 2, 255, 128, 0]);
        assert_eq!(attrs, vec![SgrAttr::Fg(Color::Rgb(255, 128, 0))]);
    }

    #[test]
    fn sgr_reset_and_bold() {
        let attrs = parse_sgr(&[0, 1]);
        assert_eq!(attrs, vec![SgrAttr::Reset, SgrAttr::Bold(true)]);
    }

    #[test]
    fn utf8_multibyte_assembles() {
        let ev = events("你好".as_bytes());
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0], AnsiEvent::Char('你'));
        assert_eq!(ev[1], AnsiEvent::Char('好'));
    }

    #[test]
    fn utf8_split_across_feeds() {
        let mut p = AnsiParser::new();
        let bytes = "你".as_bytes();
        p.feed(&bytes[..1]);
        p.feed(&bytes[1..]);
        let ev = p.drain();
        assert_eq!(ev, vec![AnsiEvent::Char('你')]);
    }

    #[test]
    fn utf8_invalid_gets_replacement() {
        let ev = events(&[0xff, 0xfe]);
        assert!(ev.contains(&AnsiEvent::InvalidUtf8));
        assert!(ev.contains(&AnsiEvent::Char('\u{fffd}')));
    }

    #[test]
    fn mixed_text_and_sequences() {
        let ev = events(b"\x1b[1;32mOK\x1b[0m\n");
        assert!(ev.len() >= 5);
        assert!(matches!(ev[0], AnsiEvent::Csi { ref params, .. } if params.len() == 2));
        assert_eq!(ev[1], AnsiEvent::Char('O'));
        assert_eq!(ev[2], AnsiEvent::Char('K'));
        assert!(matches!(ev[3], AnsiEvent::Csi { .. }));
        assert_eq!(ev[4], AnsiEvent::Lf);
    }

    #[test]
    fn charset_select_acknowledged() {
        let ev = events(b"\x1b(B");
        assert_eq!(ev.len(), 1);
        assert_eq!(
            ev[0],
            AnsiEvent::CharsetSelect {
                which: b'(',
                target: b'B'
            }
        );
    }

    #[test]
    fn dcs_strings_are_skipped() {
        let ev = events(b"\x1bP0;1q\x1b\\");
        assert!(!ev
            .iter()
            .any(|e| matches!(e, AnsiEvent::Char(c) if *c == 'P' || *c == 'q')));
    }

    #[test]
    fn incremental_feed_matches_bulk() {
        let input = b"\x1b[38;5;196mError\x1b[0m: file not found\x1b]2;title\x07";
        let bulk = events(input);
        let mut p = AnsiParser::new();
        for chunk in input.chunks(3) {
            p.feed(chunk);
        }
        let incremental = p.drain();
        assert_eq!(
            bulk, incremental,
            "incremental feeding must be identical to bulk"
        );
    }
}
