use crate::spu::SoundRam;
use crate::spu::Volume;
use crate::spu::apply_volume;
use crate::spu::clamped_i16;

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

    pub apf1_offset: usize,
    pub apf2_offset: usize,

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

    pub diff_reflection_addr1_l: usize,
    pub diff_reflection_addr1_r: usize,
    pub diff_reflection_addr2_l: usize,
    pub diff_reflection_addr2_r: usize,

    pub apf1_addr_l: usize,
    pub apf1_addr_r: usize,
    pub apf2_addr_l: usize,
    pub apf2_addr_r: usize,

    pub half_tick: bool,
    pub current_buffer_addr: usize,

    pub l_out: i16,
    pub r_out: i16,
}

impl Reverb {
    pub fn set_base_addr(&mut self, addr: u16) {
        self.base_addr = usize::from(addr) * 8;
        self.current_buffer_addr = self.base_addr;
    }

    pub fn tick(&mut self, mixed: [i32; 2], ram: &mut SoundRam, write_to_ram: bool) {
        self.half_tick = !self.half_tick;

        // The reverb hardware ticks at 22.05 KHz
        if self.half_tick {
            return;
        }

        //  ___Input from Mixer (Input volume multiplied with incoming data)_____________
        // Lin = vLIN * LeftInput    ;from any channels that have Reverb enabled
        // Rin = vRIN * RightInput   ;from any channels that have Reverb enabled

        let l_in = i32::from(apply_volume(clamped_i16(mixed[0]), self.vol_in.l));
        let r_in = i32::from(apply_volume(clamped_i16(mixed[1]), self.vol_in.r));

        // ____Same Side Reflection (left-to-left and right-to-right)___________________
        // [mLSAME] = (Lin + [dLSAME]*vWALL - [mLSAME-2])*vIIR + [mLSAME-2]  ;L-to-L
        // [mRSAME] = (Rin + [dRSAME]*vWALL - [mRSAME-2])*vIIR + [mRSAME-2]  ;R-to-R

        let m_lsame = self.wrapped_offset_sub(self.same_reflection_addr1_l, 0);
        let m_rsame = self.wrapped_offset_sub(self.same_reflection_addr1_r, 0);

        let m_lsame_sub_2 = self.wrapped_offset_sub(self.same_reflection_addr1_l, 2);
        let m_rsame_sub_2 = self.wrapped_offset_sub(self.same_reflection_addr1_r, 2);

        let d_lsame = self.wrapped_offset_sub(self.same_reflection_addr2_l, 0);
        let d_rsame = self.wrapped_offset_sub(self.same_reflection_addr2_r, 0);

        let dl_sample = i32::from(ram.read_sample(d_lsame));
        let dr_sample = i32::from(ram.read_sample(d_rsame));
        let ml_sample = i32::from(ram.read_sample(m_lsame_sub_2));
        let mr_sample = i32::from(ram.read_sample(m_rsame_sub_2));

        let v_wall = i32::from(self.wall_reflection_gain);
        let v_iir = i32::from(self.iir_reflection_gain);

        let l_to_l = mul_16(l_in + mul_16(dl_sample, v_wall) - ml_sample, v_iir) + ml_sample;
        let r_to_r = mul_16(r_in + mul_16(dr_sample, v_wall) - mr_sample, v_iir) + mr_sample;

        if write_to_ram {
            // The values written to memory are saturated to -8000h..+7FFFh.
            ram.write_sample(m_lsame, clamped_i16(l_to_l));
            ram.write_sample(m_rsame, clamped_i16(r_to_r));
        }

        // ___Different Side Reflection (left-to-right and right-to-left)_______________
        // [mLDIFF] = (Lin + [dRDIFF]*vWALL - [mLDIFF-2])*vIIR + [mLDIFF-2]  ;R-to-L
        // [mRDIFF] = (Rin + [dLDIFF]*vWALL - [mRDIFF-2])*vIIR + [mRDIFF-2]  ;L-to-R

        let m_ldiff = self.wrapped_offset_sub(self.diff_reflection_addr1_l, 0);
        let m_rdiff = self.wrapped_offset_sub(self.diff_reflection_addr1_r, 0);

        let m_ldiff_sub_2 = self.wrapped_offset_sub(self.diff_reflection_addr1_l, 2);
        let m_rdiff_sub_2 = self.wrapped_offset_sub(self.diff_reflection_addr1_r, 2);

        let d_ldiff = self.wrapped_offset_sub(self.diff_reflection_addr2_l, 0);
        let d_rdiff = self.wrapped_offset_sub(self.diff_reflection_addr2_r, 0);

        let dl_sample = i32::from(ram.read_sample(d_ldiff));
        let dr_sample = i32::from(ram.read_sample(d_rdiff));
        let ml_sample = i32::from(ram.read_sample(m_ldiff_sub_2));
        let mr_sample = i32::from(ram.read_sample(m_rdiff_sub_2));

        let v_wall = i32::from(self.wall_reflection_gain);
        let v_iir = i32::from(self.iir_reflection_gain);

        let r_to_l = mul_16(l_in + mul_16(dr_sample, v_wall) - ml_sample, v_iir) + ml_sample;
        let l_to_r = mul_16(r_in + mul_16(dl_sample, v_wall) - mr_sample, v_iir) + mr_sample;

        if write_to_ram {
            // The values written to memory are saturated to -8000h..+7FFFh.
            ram.write_sample(m_ldiff, clamped_i16(r_to_l));
            ram.write_sample(m_rdiff, clamped_i16(l_to_r));
        }

        // ___Early Echo (Comb Filter, with input from buffer)__________________________
        // Lout=vCOMB1*[mLCOMB1]+vCOMB2*[mLCOMB2]+vCOMB3*[mLCOMB3]+vCOMB4*[mLCOMB4]
        // Rout=vCOMB1*[mRCOMB1]+vCOMB2*[mRCOMB2]+vCOMB3*[mRCOMB3]+vCOMB4*[mRCOMB4]

        let ml_comb1 = ram.read_sample(self.wrapped_offset_sub(self.comb1_addr_l, 0));
        let ml_comb2 = ram.read_sample(self.wrapped_offset_sub(self.comb2_addr_l, 0));
        let ml_comb3 = ram.read_sample(self.wrapped_offset_sub(self.comb3_addr_l, 0));
        let ml_comb4 = ram.read_sample(self.wrapped_offset_sub(self.comb4_addr_l, 0));

        let mr_comb1 = ram.read_sample(self.wrapped_offset_sub(self.comb1_addr_r, 0));
        let mr_comb2 = ram.read_sample(self.wrapped_offset_sub(self.comb2_addr_r, 0));
        let mr_comb3 = ram.read_sample(self.wrapped_offset_sub(self.comb3_addr_r, 0));
        let mr_comb4 = ram.read_sample(self.wrapped_offset_sub(self.comb4_addr_r, 0));

        let mut l_out = apply_gain(self.comb1_gain, ml_comb1)
            + apply_gain(self.comb2_gain, ml_comb2)
            + apply_gain(self.comb3_gain, ml_comb3)
            + apply_gain(self.comb4_gain, ml_comb4);

        let mut r_out = apply_gain(self.comb1_gain, mr_comb1)
            + apply_gain(self.comb2_gain, mr_comb2)
            + apply_gain(self.comb3_gain, mr_comb3)
            + apply_gain(self.comb4_gain, mr_comb4);

        // ___Late Reverb APF1 (All Pass Filter 1, with input from COMB)________________
        // Lout=Lout-vAPF1*[mLAPF1-dAPF1], [mLAPF1]=Lout, Lout=Lout*vAPF1+[mLAPF1-dAPF1]
        // Rout=Rout-vAPF1*[mRAPF1-dAPF1], [mRAPF1]=Rout, Rout=Rout*vAPF1+[mRAPF1-dAPF1]

        let m_lapf1 = self.wrapped_offset_sub(self.apf1_addr_l, 0);
        let m_rapf1 = self.wrapped_offset_sub(self.apf1_addr_r, 0);
        let ml_addr = self.wrapped_offset_sub(self.apf1_addr_l, self.apf1_offset);
        let mr_addr = self.wrapped_offset_sub(self.apf1_addr_r, self.apf1_offset);

        l_out -= apply_gain(self.apf1_gain, ram.read_sample(ml_addr));
        r_out -= apply_gain(self.apf1_gain, ram.read_sample(mr_addr));

        let l_out = clamped_i16(l_out);
        let r_out = clamped_i16(r_out);

        if write_to_ram {
            // The values written to memory are saturated to -8000h..+7FFFh.
            ram.write_sample(m_lapf1, l_out);
            ram.write_sample(m_rapf1, r_out);
        }

        let mut l_out = apply_gain(self.apf1_gain, l_out) + i32::from(ram.read_sample(ml_addr));
        let mut r_out = apply_gain(self.apf1_gain, r_out) + i32::from(ram.read_sample(mr_addr));

        //   ___Late Reverb APF2 (All Pass Filter 2, with input from APF1)________________
        // Lout=Lout-vAPF2*[mLAPF2-dAPF2], [mLAPF2]=Lout, Lout=Lout*vAPF2+[mLAPF2-dAPF2]
        // Rout=Rout-vAPF2*[mRAPF2-dAPF2], [mRAPF2]=Rout, Rout=Rout*vAPF2+[mRAPF2-dAPF2]

        let m_lapf2 = self.wrapped_offset_sub(self.apf2_addr_l, 0);
        let m_rapf2 = self.wrapped_offset_sub(self.apf2_addr_r, 0);
        let ml_addr = self.wrapped_offset_sub(self.apf2_addr_l, self.apf2_offset);
        let mr_addr = self.wrapped_offset_sub(self.apf2_addr_r, self.apf2_offset);

        l_out -= apply_gain(self.apf2_gain, ram.read_sample(ml_addr));
        r_out -= apply_gain(self.apf2_gain, ram.read_sample(mr_addr));

        let l_out = clamped_i16(l_out);
        let r_out = clamped_i16(r_out);

        if write_to_ram {
            // The values written to memory are saturated to -8000h..+7FFFh.
            ram.write_sample(m_lapf2, l_out);
            ram.write_sample(m_rapf2, r_out);
        }

        let l_out = apply_gain(self.apf2_gain, l_out) + i32::from(ram.read_sample(ml_addr));
        let r_out = apply_gain(self.apf2_gain, r_out) + i32::from(ram.read_sample(mr_addr));

        let l_out = clamped_i16(l_out);
        let r_out = clamped_i16(r_out);

        // ___Output to Mixer (Output volume multiplied with input from APF2)___________
        // LeftOutput  = Lout*vLOUT
        // RightOutput = Rout*vROUT

        self.l_out = apply_volume(l_out, self.vol_out.l);
        self.r_out = apply_volume(r_out, self.vol_out.r);

        // BufferAddress = MAX(mBASE, (BufferAddress+2) AND 7FFFEh)
        self.current_buffer_addr = ((self.current_buffer_addr + 2) & 0x7FFFE).max(self.base_addr);
    }

    /// All memory addresses are relative to `current_buffer_addr`,
    /// and wrapped within `base_addr`..=0x7FFFE when exceeding that region.
    const fn wrapped_offset_sub(&self, pos: usize, sub: usize) -> usize {
        let base = self.base_addr;
        let curr = self.current_buffer_addr;
        let range = 0x7FFFE - base + 1;

        let rel = (curr + range - base) % range;
        let offset = (rel + pos + range - (sub % range)) % range;

        base + offset
    }
}

/// The multiplication results are divided by +8000h, to fit them to 16bit range.
const fn mul_16(a: i32, b: i32) -> i32 {
    (a * b) >> 15
}

fn apply_gain(volume: i16, sample: i16) -> i32 {
    (i32::from(volume) * i32::from(sample)) >> 15
}
