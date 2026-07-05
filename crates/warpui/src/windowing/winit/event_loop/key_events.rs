use std::borrow::Cow;

use winit::event::ElementState;
#[cfg(windows)]
use winit::keyboard::NativeKey;
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

use crate::platform::KEYS_TO_IGNORE;
use crate::{event::KeyEventDetails, keymap::Keystroke};

use super::WindowState;

/// Converts a KeyboardInput event to a UI framework event, returning None
/// if no UI framework event should be emitted.
pub fn convert_keyboard_input_event(
    input: winit::event::KeyEvent,
    window_state: &WindowState,
    is_synthetic: bool,
) -> Option<crate::Event> {
    if input.state != ElementState::Pressed {
        return None;
    }

    // Ignore any synthetic keypresses that winit generated for keys that were
    // already pressed when a window gained focus.  Three examples of how these
    // cause problems:
    // 1. An alt-tab to a window can end up inserting a tab into the input if
    //    alt is released before tab.
    // 2. Using a keyboard shortcut to open a new window can open many new
    //    windows, as the new window will receive a synthetic event for the
    //    shortcut that opened it, opening _another_ new window, and so on.
    // 3. The ctrl-d shortcut for sending an EOF to the shell can end up
    //    being sent to additional sessions if there was ony one session in
    //    the window, as it will close the window and then be synthetically
    //    generated for the next window in the stack.
    if is_synthetic {
        return None;
    }

    let chars = text_with_modifiers(&input, window_state.modifiers)
        .unwrap_or_default()
        .to_owned();

    let key_without_modifiers = get_key_without_modifiers(&input);

    let shift = window_state.modifiers.shift_key();

    let logical_key = match &input.logical_key {
        // When keystrokes with ctrl-alt are pressed on Windows, `input.logical_key` is
        // Unidentified.
        #[cfg(windows)]
        Key::Unidentified(NativeKey::Windows(_))
            if window_state
                .modifiers
                .contains(ModifiersState::CONTROL | ModifiersState::ALT) =>
        {
            input.key_without_modifiers()
        }
        _ => input.logical_key,
    };
    let input_key = get_input_key(&logical_key, shift);

    let key = convert_key(input_key)?.to_string();

    let keystroke = Keystroke {
        ctrl: window_state.modifiers.control_key(),
        alt: window_state.modifiers.alt_key(),
        shift,
        cmd: window_state.modifiers.super_key(),
        meta: false,
        key,
    };

    // Ignore any keystrokes that we're purposefully not handling. (I.e. cmdorctrl-v needs to fall back
    // to the browser implementation on the web.)
    if KEYS_TO_IGNORE.contains(&keystroke) {
        return None;
    }

    Some(crate::event::Event::KeyDown {
        keystroke,
        chars,
        details: KeyEventDetails {
            left_alt: window_state.left_alt_pressed,
            right_alt: window_state.right_alt_pressed,
            key_without_modifiers,
        },
        is_composing: false,
    })
}
/// Returns the base key without any modifiers applied, or `None` if it cannot be determined.
fn get_key_without_modifiers(input: &winit::event::KeyEvent) -> Option<String> {
    let unmodified = input.key_without_modifiers();
    let unmodified_input = get_input_key(&unmodified, false);
    convert_key(unmodified_input).map(|k| k.to_string())
}
/// Returns the text of the [`winit::event::KeyEvent`] with the characters modified by `ctrl`.
/// For example,  `Ctrl+a` produces `Some("\x01")`.
fn text_with_modifiers(
    key_event: &winit::event::KeyEvent,
    _modifier_state: ModifiersState,
) -> Option<&str> {
    key_event.text_with_all_modifiers()
}

fn get_input_key(logical_key: &Key, is_shift: bool) -> Key {
    use winit::keyboard::Key::Character;
    match (logical_key, is_shift) {
        // If the key is a character AND shift is pressed, we force the key to uppercase.
        // If the key is a character AND shift is NOT pressed, we force the key to lowercase.
        // This is to align with existing behavior where we expect bindings with shift
        // to have uppercase characters, and bindings without shift to have lowercase characters.
        // See warpui::keymap::Keystroke::parse and warp::util::bindings::cmd_or_ctrl_shift.
        (Character(character), true) => Character(character.to_uppercase().into()),
        (Character(character), false) => Character(character.to_lowercase().into()),
        (non_char_key, _) => non_char_key.clone(),
    }
}

/// Converts a winit [`winit::keyboard::Key`] to the corresponding string version
/// expected by the UI framework.
fn convert_key(key: Key) -> Option<Cow<'static, str>> {
    use winit::keyboard::Key::*;

    let value = match key {
        Character(char) => return Some(char.to_string().into()),
        Named(NamedKey::Enter) => "enter",
        Named(NamedKey::Tab) => "tab",
        Named(NamedKey::Space) => " ",
        Named(NamedKey::ArrowDown) => "down",
        Named(NamedKey::ArrowLeft) => "left",
        Named(NamedKey::ArrowRight) => "right",
        Named(NamedKey::ArrowUp) => "up",
        Named(NamedKey::End) => "end",
        Named(NamedKey::Home) => "home",
        Named(NamedKey::PageDown) => "pagedown",
        Named(NamedKey::PageUp) => "pageup",
        Named(NamedKey::Backspace) => "backspace",
        Named(NamedKey::Delete) => "delete",
        Named(NamedKey::Insert) => "insert",
        Named(NamedKey::Escape) => "escape",
        Named(NamedKey::F1) => "f1",
        Named(NamedKey::F2) => "f2",
        Named(NamedKey::F3) => "f3",
        Named(NamedKey::F4) => "f4",
        Named(NamedKey::F5) => "f5",
        Named(NamedKey::F6) => "f6",
        Named(NamedKey::F7) => "f7",
        Named(NamedKey::F8) => "f8",
        Named(NamedKey::F9) => "f9",
        Named(NamedKey::F10) => "f10",
        Named(NamedKey::F11) => "f11",
        Named(NamedKey::F12) => "f12",
        Named(NamedKey::F13) => "f13",
        Named(NamedKey::F14) => "f14",
        Named(NamedKey::F15) => "f15",
        Named(NamedKey::F16) => "f16",
        Named(NamedKey::F17) => "f17",
        Named(NamedKey::F18) => "f18",
        Named(NamedKey::F19) => "f19",
        Named(NamedKey::F20) => "f20",
        Named(NamedKey::F21) => "f21",
        Named(NamedKey::F22) => "f22",
        Named(NamedKey::F23) => "f23",
        Named(NamedKey::F24) => "f24",
        Named(NamedKey::F25) => "f25",
        Named(NamedKey::F26) => "f26",
        Named(NamedKey::F27) => "f27",
        Named(NamedKey::F28) => "f28",
        Named(NamedKey::F29) => "f29",
        Named(NamedKey::F30) => "f30",
        Named(NamedKey::F31) => "f31",
        Named(NamedKey::F32) => "f32",
        Named(NamedKey::F33) => "f33",
        Named(NamedKey::F34) => "f34",
        Named(NamedKey::F35) => "f35",
        _ => return None,
    };

    Some(Cow::Borrowed(value))
}

#[cfg(test)]
#[path = "key_events_tests.rs"]
mod tests;
