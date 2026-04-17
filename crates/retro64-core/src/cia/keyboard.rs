//! 8×8 keyboard matrix.

/// A C64 key position identified by (column, row).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum C64Key {
    /// INS/DEL key.
    Delete, Return, CursorRight, F7, F1, F3, F5, CursorDown,
    /// Top digit row.
    Num3, W, A, Num4, Z, S, E, LShift,
    /// Second row.
    Num5, R, D, Num6, C, F, T, X,
    /// Middle row.
    Num7, Y, G, Num8, B, H, U, V,
    /// Lower middle row.
    Num9, I, J, Num0, M, K, O, N,
    /// Punctuation row.
    Plus, P, L, Minus, Period, Colon, At, Comma,
    /// Upper punctuation row.
    Pound, Asterisk, Semicolon, Home, RShift, Equals, ArrowUp, Slash,
    /// Control row.
    Num1, ArrowLeft, Control, Num2, Space, Commodore, Q, RunStop,
}

/// Modifier-only keys that don't live in the matrix proper.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Modifier {
    /// Triggers an NMI (RESTORE).
    Restore,
}

/// (column, row) for each [`C64Key`] in the 8×8 matrix.
pub fn matrix_pos(k: C64Key) -> (u8, u8) {
    use C64Key::*;
    match k {
        Delete=>(0,0), Return=>(1,0), CursorRight=>(2,0), F7=>(3,0),
        F1=>(4,0), F3=>(5,0), F5=>(6,0), CursorDown=>(7,0),
        Num3=>(0,1), W=>(1,1), A=>(2,1), Num4=>(3,1),
        Z=>(4,1), S=>(5,1), E=>(6,1), LShift=>(7,1),
        Num5=>(0,2), R=>(1,2), D=>(2,2), Num6=>(3,2),
        C=>(4,2), F=>(5,2), T=>(6,2), X=>(7,2),
        Num7=>(0,3), Y=>(1,3), G=>(2,3), Num8=>(3,3),
        B=>(4,3), H=>(5,3), U=>(6,3), V=>(7,3),
        Num9=>(0,4), I=>(1,4), J=>(2,4), Num0=>(3,4),
        M=>(4,4), K=>(5,4), O=>(6,4), N=>(7,4),
        Plus=>(0,5), P=>(1,5), L=>(2,5), Minus=>(3,5),
        Period=>(4,5), Colon=>(5,5), At=>(6,5), Comma=>(7,5),
        Pound=>(0,6), Asterisk=>(1,6), Semicolon=>(2,6), Home=>(3,6),
        RShift=>(4,6), Equals=>(5,6), ArrowUp=>(6,6), Slash=>(7,6),
        Num1=>(0,7), ArrowLeft=>(1,7), Control=>(2,7), Num2=>(3,7),
        Space=>(4,7), Commodore=>(5,7), Q=>(6,7), RunStop=>(7,7),
    }
}

/// 8×8 keyboard matrix (active-low: pressed key zeroes the intersection).
#[derive(Default)]
pub struct KeyboardMatrix {
    /// One byte per row; bit N set = column N pressed.
    columns_by_row: [u8; 8],
}

impl KeyboardMatrix {
    /// New empty matrix.
    pub fn new() -> Self { KeyboardMatrix { columns_by_row: [0; 8] } }

    /// Press a key.
    pub fn press(&mut self, k: C64Key) {
        let (c, r) = matrix_pos(k);
        self.columns_by_row[r as usize] |= 1 << c;
    }

    /// Release a key.
    pub fn release(&mut self, k: C64Key) {
        let (c, r) = matrix_pos(k);
        self.columns_by_row[r as usize] &= !(1 << c);
    }

    /// Read Port B given the column-select mask on Port A (active-low).
    /// Returns 0xFF with bits cleared for rows containing a pressed key
    /// in a selected column.
    pub fn read_pb(&self, col_select: u8) -> u8 {
        let cols_active = !col_select;
        let mut out: u8 = 0xFF;
        for (r, &cols) in self.columns_by_row.iter().enumerate() {
            if cols & cols_active != 0 {
                out &= !(1 << r);
            }
        }
        out
    }
}
