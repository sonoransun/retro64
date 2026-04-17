//! VIC-II 16-color Pepto palette in ARGB8888.

/// 16-colour Pepto-like palette.
pub const VIC_PALETTE: [u32; 16] = [
    0xFF000000, // 0  black
    0xFFFFFFFF, // 1  white
    0xFF880000, // 2  red
    0xFFAAFFEE, // 3  cyan
    0xFFCC44CC, // 4  purple
    0xFF00CC55, // 5  green
    0xFF0000AA, // 6  blue
    0xFFEEEE77, // 7  yellow
    0xFFDD8855, // 8  orange
    0xFF664400, // 9  brown
    0xFFFF7777, // 10 light red
    0xFF333333, // 11 dark grey
    0xFF777777, // 12 medium grey
    0xFFAAFF66, // 13 light green
    0xFF0088FF, // 14 light blue
    0xFFBBBBBB, // 15 light grey
];
