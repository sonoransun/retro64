//! Joystick state (5-bit: up/down/left/right/fire, active-low).

/// Joystick port state. Bits set = direction/fire active.
#[derive(Default, Copy, Clone, Debug)]
pub struct JoystickState {
    /// Bit 0=up, 1=down, 2=left, 3=right, 4=fire.
    pub bits: u8,
}

impl JoystickState {
    /// Convert to an active-low port value.
    pub fn as_port(&self) -> u8 {
        !self.bits & 0x1F | 0xE0
    }
}
