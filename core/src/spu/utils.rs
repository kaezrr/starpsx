pub fn write_half<const HIGH: bool>(reg: &mut u32, val: u16) {
    let shift = if HIGH { 16 } else { 0 };
    let mask = 0xFFFF << shift;

    *reg = (*reg & !mask) | ((val as u32) << shift);
}
