use super::*;

pub fn matrix_reg_read(m: &[[i16; 3]; 3], r: usize) -> u32 {
    debug_assert!(r <= 4);
    if r == 4 {
        return m[2][2] as u32;
    }

    let elems = m.as_flattened();
    let msb = elems[r * 2 + 1] as u32;
    let lsb = elems[r * 2] as u32;

    (msb << 16) | (lsb & 0xFFFF)
}

pub fn matrix_reg_write(m: &mut [[i16; 3]; 3], r: usize, v: u32) {
    debug_assert!(r <= 4);
    if r == 4 {
        m[2][2] = (v & 0xFFFF) as i16;
        return;
    }

    let elems = m.as_flattened_mut();
    elems[r * 2 + 1] = (v >> 16) as i16;
    elems[r * 2] = (v & 0xFFFF) as i16;
}

pub fn vec_xy_read(v: &[i16]) -> u32 {
    let lsb = v[0] as u32;
    let msb = v[1] as u32;
    msb << 16 | (lsb & 0xFFFF)
}

pub fn vec_xy_write(v: &mut [i16], d: u32) {
    v[0] = (d & 0xFFFF) as i16;
    v[1] = (d >> 16) as i16;
}
