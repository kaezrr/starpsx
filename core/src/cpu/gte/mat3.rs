#[derive(Default, Debug)]
pub struct Matrix3 {
    elems: [i16; 9],
}

impl Matrix3 {
    pub fn write_reg_u32(&mut self, r: usize, v: u32) {
        if r == 4 {
            self.elems[8] = (v & 0xFFFF) as i16;
            return;
        }

        self.elems[r * 2 + 1] = (v >> 16) as i16;
        self.elems[r * 2] = (v & 0xFFFF) as i16;
    }

    pub fn as_reg_u32(&self, r: usize) -> u32 {
        if r == 4 {
            return self.elems[8] as u32;
        }

        let msb = self.elems[r * 2 + 1] as u32;
        let lsb = self.elems[r * 2] as u32;

        (msb << 16) | (lsb & 0xFFFF)
    }

    pub fn at(&self, r: usize, c: usize) -> i16 {
        let i = r * 3 + c;

        debug_assert!(i < 9);

        self.elems[i]
    }
}
