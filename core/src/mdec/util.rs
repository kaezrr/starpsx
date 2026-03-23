pub const fn signed10bit(v: u16) -> i16 {
    (v as i16) << 6 >> 6
}

// Reversed zig zag table
pub const ZAG_ZIG: [usize; 64] = {
    let mut buf = [0; 64];
    let mut i = 0;
    while i < 64 {
        buf[ZIG_ZAG[i]] = i;
        i += 1;
    }
    buf
};

const ZIG_ZAG: [usize; 64] = [
    0, 1, 5, 6, 14, 15, 27, 28, 2, 4, 7, 13, 16, 26, 29, 42, 3, 8, 12, 17, 25, 30, 41, 43, 9, 11,
    18, 24, 31, 40, 44, 53, 10, 19, 23, 32, 39, 45, 52, 54, 20, 22, 33, 38, 46, 51, 55, 60, 21, 34,
    37, 47, 50, 56, 59, 61, 35, 36, 48, 49, 57, 58, 62, 63,
];

pub fn level_shift_8bpp(block: [i16; 64]) -> [u8; 64] {
    let mut out = [0; 64];
    for (i, &y) in block.iter().enumerate() {
        let masked = y & 0x1FF;
        let signed_9bit = if masked > 0xFF {
            masked | !0x1FF
        } else {
            masked
        };

        let clamped = signed_9bit.clamp(-128, 127) as i8;
        out[i] = (clamped as u8) ^ 0x80;
    }
    out
}

pub fn level_shift_4bpp(block: [i16; 64]) -> [u8; 32] {
    let pixels = level_shift_8bpp(block);
    let mut out = [0u8; 32];
    for (i, y) in pixels.chunks_exact(2).enumerate() {
        out[i] = (y[0] >> 4) | (y[1] & 0xF0);
    }
    out
}

pub fn yuv_to_rgb15_block(
    cr: &[i16; 64],
    cb: &[i16; 64],
    y: &[i16; 64],
    pos: (usize, usize),
    is_signed: bool,
    b15: bool,
    dst: &mut [u16; 256],
) {
    let (xx, yy) = pos;
    for py in 0..8 {
        for px in 0..8 {
            let cr_val = cr[((px + xx) / 2) + ((py + yy) / 2) * 8] as i32;
            let cb_val = cb[((px + xx) / 2) + ((py + yy) / 2) * 8] as i32;

            let r_off = (1.402 * cr_val as f64) as i32;
            let b_off = (1.772 * cb_val as f64) as i32;
            let g_off = (-0.3437 * cb_val as f64 + -0.7143 * cr_val as f64) as i32;

            let luma = y[px + py * 8] as i32;

            let mut r = (luma + r_off).clamp(-128, 127);
            let mut g = (luma + g_off).clamp(-128, 127);
            let mut b = (luma + b_off).clamp(-128, 127);

            if !is_signed {
                r ^= 0x80;
                g ^= 0x80;
                b ^= 0x80;
            }

            let r5 = (r as u8 >> 3) as u16;
            let g5 = (g as u8 >> 3) as u16;
            let b5 = (b as u8 >> 3) as u16;
            let pixel = r5 | (g5 << 5) | (b5 << 10) | (b15 as u16) << 15;

            dst[(px + xx) + (py + yy) * 16] = pixel;
        }
    }
}
