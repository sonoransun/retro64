//! `KeyboardEvent.code` → C64Key mapping.

use retro64_core::C64Key;

pub fn map_code(code: &str) -> Option<C64Key> {
    use C64Key::*;
    Some(match code {
        "KeyA" => A, "KeyB" => B, "KeyC" => C, "KeyD" => D, "KeyE" => E,
        "KeyF" => F, "KeyG" => G, "KeyH" => H, "KeyI" => I, "KeyJ" => J,
        "KeyK" => K, "KeyL" => L, "KeyM" => M, "KeyN" => N, "KeyO" => O,
        "KeyP" => P, "KeyQ" => Q, "KeyR" => R, "KeyS" => S, "KeyT" => T,
        "KeyU" => U, "KeyV" => V, "KeyW" => W, "KeyX" => X, "KeyY" => Y, "KeyZ" => Z,
        "Digit0" => Num0, "Digit1" => Num1, "Digit2" => Num2, "Digit3" => Num3,
        "Digit4" => Num4, "Digit5" => Num5, "Digit6" => Num6, "Digit7" => Num7,
        "Digit8" => Num8, "Digit9" => Num9,
        "Space" => Space, "Enter" => Return,
        "Backspace" | "Delete" => Delete,
        "ShiftLeft" => LShift, "ShiftRight" => RShift,
        "ControlLeft" | "ControlRight" => Control,
        "Escape" => RunStop,
        "Period" => Period, "Comma" => Comma,
        "Semicolon" => Semicolon, "Slash" => Slash,
        "Minus" => Minus, "Equal" => Equals,
        "BracketLeft" => At, "BracketRight" => Asterisk,
        "Backslash" => Pound, "Quote" => Colon,
        "Home" => Home,
        "ArrowRight" => CursorRight, "ArrowDown" => CursorDown,
        "F1" => F1, "F3" => F3, "F5" => F5, "F7" => F7,
        "MetaLeft" | "MetaRight" => Commodore,
        _ => return None,
    })
}
