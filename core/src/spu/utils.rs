pub fn write_half<const HIGH: bool>(reg: &mut u32, val: u16) {
    let shift = if HIGH { 16 } else { 0 };
    let mask = 0xFFFF << shift;

    *reg = (*reg & !mask) | ((val as u32) << shift);
}

pub fn decode_adpcm_block(
    block: &[u8; 16],
    decoded: &mut [i16; 28],
    old_sample: &mut i16,
    older_sample: &mut i16,
) {
    let shift = block[0] & 0x0F;
    let shift = if shift > 12 { 9 } else { shift };

    let filter = ((block[0] >> 4) & 0x07).min(4);

    for sample_idx in 0..28 {
        let sample_byte = block[2 + sample_idx / 2];
        let sample_nibble = (sample_byte >> (4 * (sample_idx % 2))) & 0x0F;

        let raw_sample: i32 = (((sample_nibble as i8) << 4) >> 4).into();

        let shifted_sample = raw_sample << (12 - shift);

        let old = i32::from(*old_sample);
        let older = i32::from(*older_sample);
        let filtered_sample = match filter {
            0 => shifted_sample,
            1 => shifted_sample + (60 * old + 32) / 64,
            2 => shifted_sample + (115 * old - 52 * older + 32) / 64,
            3 => shifted_sample + (98 * old - 55 * older + 32) / 64,
            4 => shifted_sample + (122 * old - 60 * older + 32) / 64,
            _ => unreachable!("filter was clamped to 0..=4"),
        };

        let clamped_sample = filtered_sample.clamp(-0x8000, 0x7FFF) as i16;
        decoded[sample_idx] = clamped_sample;

        *older_sample = *old_sample;
        *old_sample = clamped_sample;
    }
}
