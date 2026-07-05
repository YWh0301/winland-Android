/// Arabic (RTL) text input support for the Wayland compositor.
///
/// When `TextCommit` events contain characters not present in the xkb keymap
/// (e.g. Arabic letters U+0600–U+06FF), the keycode-based injection path in
/// `inject_text_commit()` drops them silently. This module provides an
/// alternative path using the `text-input` Wayland protocol, bypassing xkb.

#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::seat::AndroidSeatRuntime;
#[cfg(feature = "smithay_android")]
use smithay::wayland::text_input::TextInputSeat;

/// Whether `text` contains characters outside the Latin-1 range that will
/// never match the `"us"` xkb keymap and therefore need the text‑input
/// protocol path.
#[cfg(feature = "smithay_android")]
pub fn needs_text_input_protocol(text: &str) -> bool {
    text.contains(|c: char| {
        match c {
            // Arabic & RTL blocks (the primary target)
            '\u{0600}'..='\u{06FF}'  // Arabic
            | '\u{0750}'..='\u{077F}'  // Arabic Supplement
            | '\u{08A0}'..='\u{08FF}'  // Arabic Extended-A
            | '\u{0870}'..='\u{089F}'  // Arabic Extended-B
            | '\u{FB50}'..='\u{FDFF}'  // Arabic Pres. Forms-A
            | '\u{FE70}'..='\u{FEFF}'  // Arabic Pres. Forms-B
            | '\u{1EE00}'..='\u{1EEFF}' // Arabic Mathematical
            // Common non-Latin scripts absent from a "us" keymap
            | '\u{0400}'..='\u{04FF}'  // Cyrillic
            | '\u{0590}'..='\u{05FF}'  // Hebrew
            | '\u{0E00}'..='\u{0E7F}'  // Thai
            | '\u{4E00}'..='\u{9FFF}'  // CJK Unified
            | '\u{AC00}'..='\u{D7AF}'  // Hangul Syllables
            | '\u{1100}'..='\u{11FF}'  // Hangul Jamo
            | '\u{3040}'..='\u{309F}'  // Hiragana
            | '\u{30A0}'..='\u{30FF}'  // Katakana
            | '\u{0700}'..='\u{074F}'  // Syriac
            | '\u{0900}'..='\u{097F}'  // Devanagari
            | '\u{1B00}'..='\u{1B7F}'  // Balinese
            | '\u{1780}'..='\u{17FF}'  // Khmer
            => true,
            _ => false,
        }
    })
}

/// Commit `text` via the `text‑input` Wayland protocol, bypassing keycode
/// mapping.  Returns `true` when a client had an active text‑input and the
/// text was forwarded.
#[cfg(feature = "smithay_android")]
pub fn commit_text_via_protocol(runtime: &mut AndroidSeatRuntime, text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    let text_input = runtime.seat.text_input();
    let mut committed = false;

    text_input.with_active_text_input(|ti, _surface| {
        ti.commit_string(Some(text.to_string()));
        committed = true;
    });

    if committed {
        text_input.done(false);
        log::info!(
            "arabic_input: committed {} chars via text‑input protocol",
            text.chars().count(),
        );
    } else {
        log::warn!(
            "arabic_input: no active text‑input for text {:?} — falling back",
            text,
        );
    }

    committed
}
