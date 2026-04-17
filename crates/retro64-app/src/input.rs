//! SDL Keycode -> C64Key mapping.

use retro64_core::C64Key;
use sdl2::keyboard::Keycode;

pub fn map(kc: Keycode) -> Option<C64Key> {
    use C64Key::*;
    Some(match kc {
        Keycode::A => A, Keycode::B => B, Keycode::C => C, Keycode::D => D,
        Keycode::E => E, Keycode::F => F, Keycode::G => G, Keycode::H => H,
        Keycode::I => I, Keycode::J => J, Keycode::K => K, Keycode::L => L,
        Keycode::M => M, Keycode::N => N, Keycode::O => O, Keycode::P => P,
        Keycode::Q => Q, Keycode::R => R, Keycode::S => S, Keycode::T => T,
        Keycode::U => U, Keycode::V => V, Keycode::W => W, Keycode::X => X,
        Keycode::Y => Y, Keycode::Z => Z,
        Keycode::Num0 => Num0, Keycode::Num1 => Num1, Keycode::Num2 => Num2,
        Keycode::Num3 => Num3, Keycode::Num4 => Num4, Keycode::Num5 => Num5,
        Keycode::Num6 => Num6, Keycode::Num7 => Num7, Keycode::Num8 => Num8,
        Keycode::Num9 => Num9,
        Keycode::Space => Space,
        Keycode::Return => Return,
        Keycode::Backspace | Keycode::Delete => Delete,
        Keycode::LShift => LShift,
        Keycode::RShift => RShift,
        Keycode::LCtrl | Keycode::RCtrl => Control,
        Keycode::Escape => RunStop,
        Keycode::Tab => Control,
        Keycode::Period => Period,
        Keycode::Comma => Comma,
        Keycode::Semicolon => Semicolon,
        Keycode::Slash => Slash,
        Keycode::Minus => Minus,
        Keycode::Equals => Equals,
        Keycode::Plus => Plus,
        Keycode::Colon => Colon,
        Keycode::Home => Home,
        Keycode::Right => CursorRight,
        Keycode::Down => CursorDown,
        Keycode::F1 => F1, Keycode::F3 => F3, Keycode::F5 => F5, Keycode::F7 => F7,
        Keycode::LGui | Keycode::RGui => Commodore,
        _ => return None,
    })
}
