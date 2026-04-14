use crate::spu::Volume;

/// All volume registers are signed 16bit (range -8000h..+7FFFh).
/// All src/dst/disp/base registers are addresses in SPU memory (divided by 8),
/// src/dst are relative to the current buffer address, the disp registers are relative to src registers,
/// the base register defines the start address of the reverb buffer (the end address is fixed, at 7FFFEh).
/// Writing a value to mBASE does additionally set the current buffer address to that value.
#[derive(Default)]
pub struct Reverb {
    pub vol_in: Volume<i16>,
    pub vol_out: Volume<i16>,

    pub base_addr: usize,

    pub apf1_delay: usize,
    pub apf2_delay: usize,

    pub iir_reflection_gain: i16,

    pub comb1_gain: i16,
    pub comb2_gain: i16,
    pub comb3_gain: i16,
    pub comb4_gain: i16,

    pub wall_reflection_gain: i16,

    pub apf1_gain: i16,
    pub apf2_gain: i16,

    pub same_reflection_addr1_l: usize,
    pub same_reflection_addr1_r: usize,
    pub same_reflection_addr2_l: usize,
    pub same_reflection_addr2_r: usize,

    pub comb1_addr_l: usize,
    pub comb1_addr_r: usize,
    pub comb2_addr_l: usize,
    pub comb2_addr_r: usize,
    pub comb3_addr_l: usize,
    pub comb3_addr_r: usize,
    pub comb4_addr_l: usize,
    pub comb4_addr_r: usize,

    pub cross_reflection_addr1_l: usize,
    pub cross_reflection_addr1_r: usize,
    pub cross_reflection_addr2_l: usize,
    pub cross_reflection_addr2_r: usize,

    pub apf1_addr_l: usize,
    pub apf1_addr_r: usize,
    pub apf2_addr_l: usize,
    pub apf2_addr_r: usize,
}
