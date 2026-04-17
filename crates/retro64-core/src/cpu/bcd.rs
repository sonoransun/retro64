//! Decimal-mode ADC/SBC helpers.
//!
//! Matches the NMOS 6502 behaviour including the idiosyncratic N/V/Z flag
//! handling (BASIC's floating-point routines depend on this).

/// Decimal-mode ADC.
pub fn adc_decimal(a: u8, v: u8, carry_in: bool) -> (u8, bool, bool) {
    let ci = carry_in as u16;
    let mut lo = (a & 0x0F) as u16 + (v & 0x0F) as u16 + ci;
    if lo > 9 { lo += 6; }
    let mut hi = (a >> 4) as u16 + (v >> 4) as u16 + (lo >> 4);
    let hi_pre = hi as u8;
    let mut result = ((hi << 4) | (lo & 0x0F)) as u8;
    let v_overflow = (((a ^ result) & !(a ^ v)) & 0x80) != 0;
    if hi > 9 { hi += 6; }
    result = ((hi << 4) | (lo & 0x0F)) as u8;
    let c = hi > 0x0F;
    let _ = hi_pre;
    (result, c, v_overflow)
}

/// Decimal-mode SBC.
pub fn sbc_decimal(a: u8, v: u8, carry_in: bool) -> (u8, bool, bool) {
    // Binary result drives the flags on NMOS.
    let cin = carry_in as i16;
    let bin = (a as i16).wrapping_sub(v as i16).wrapping_sub(1 - cin);

    let mut lo = (a & 0x0F) as i16 - (v & 0x0F) as i16 - (1 - cin);
    if lo & 0x10 != 0 { lo -= 6; }
    let mut hi = (a >> 4) as i16 - (v >> 4) as i16 - ((lo & 0x10) >> 4);
    if hi & 0x10 != 0 { hi -= 6; }

    let result = (((hi as u16) << 4) | (lo as u16 & 0x0F)) as u8;
    let c = bin >= 0;
    let v_overflow = (((a ^ v) & (a ^ (bin as u8))) & 0x80) != 0;
    (result, c, v_overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_bcd_add() {
        // 15 + 27 = 42 BCD, no carry
        let (r, c, _) = adc_decimal(0x15, 0x27, false);
        assert_eq!(r, 0x42);
        assert!(!c);
    }

    #[test]
    fn bcd_add_with_carry_out() {
        // 99 + 01 + 0 = 00 with carry
        let (r, c, _) = adc_decimal(0x99, 0x01, false);
        assert_eq!(r, 0x00);
        assert!(c);
    }

    #[test]
    fn simple_bcd_sub() {
        // 42 - 15 = 27 (carry=1 means no borrow)
        let (r, c, _) = sbc_decimal(0x42, 0x15, true);
        assert_eq!(r, 0x27);
        assert!(c);
    }
}
