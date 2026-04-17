//! Status flag definitions.

use bitflags::bitflags;

bitflags! {
    /// Status register bits.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Flags: u8 {
        /// Carry.
        const C = 0x01;
        /// Zero.
        const Z = 0x02;
        /// Interrupt disable.
        const I = 0x04;
        /// Decimal mode.
        const D = 0x08;
        /// Break (only set on stack copies).
        const B = 0x10;
        /// Unused (always reads as 1).
        const U = 0x20;
        /// Overflow.
        const V = 0x40;
        /// Negative.
        const N = 0x80;
    }
}
