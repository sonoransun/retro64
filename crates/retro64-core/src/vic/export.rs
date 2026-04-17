//! Framebuffer export helpers (BMP + ARGB→RGBA).

/// Convert an ARGB8888 framebuffer to RGBA8888 bytes (for Canvas ImageData).
pub fn argb_to_rgba(fb: &[u32], out: &mut [u8]) {
    assert_eq!(out.len(), fb.len() * 4);
    for (i, p) in fb.iter().enumerate() {
        let a = (p >> 24) as u8;
        let r = (p >> 16) as u8;
        let g = (p >> 8) as u8;
        let b = *p as u8;
        out[i * 4 + 0] = r;
        out[i * 4 + 1] = g;
        out[i * 4 + 2] = b;
        out[i * 4 + 3] = a;
    }
}

/// Encode an ARGB8888 framebuffer to a 24-bit BMP file.
pub fn framebuffer_to_bmp(fb: &[u32], width: u32, height: u32) -> Vec<u8> {
    let row_bytes = ((width * 3 + 3) / 4) * 4;
    let pixel_bytes = row_bytes * height;
    let file_size = 54 + pixel_bytes;
    let mut out = Vec::with_capacity(file_size as usize);
    // BITMAPFILEHEADER
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54u32.to_le_bytes());
    // BITMAPINFOHEADER (40 bytes)
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&pixel_bytes.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    // Pixel data (BMP is bottom-up)
    for y in (0..height).rev() {
        let row_start = (y * width) as usize;
        for x in 0..width as usize {
            let p = fb[row_start + x];
            out.push(p as u8);        // B
            out.push((p >> 8) as u8); // G
            out.push((p >> 16) as u8); // R
        }
        let padding = row_bytes - width * 3;
        for _ in 0..padding { out.push(0); }
    }
    out
}
