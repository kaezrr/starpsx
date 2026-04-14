use crate::spu::SoundRam;
use crate::spu::Volume;
use crate::spu::clamped_i16;

/// All volume registers are signed 16bit (range -8000h..+7FFFh).
/// All src/dst/disp/base registers are addresses in SPU memory (divided by 8),
/// src/dst are relative to the current buffer address, the disp registers are relative to src registers,
/// the base register defines the start address of the reverb buffer (the end address is fixed, at 7FFFEh).
/// Writing a value to mBASE does additionally set the current buffer address to that value.
#[derive(Default)]
pub struct Reverb {
    pub v_out: Volume<i16>,
    pub v_in: Volume<i16>,

    pub m_base: usize,

    pub d_apf1: usize,
    pub d_apf2: usize,

    pub v_iir: i32,

    pub v_comb1: i32,
    pub v_comb2: i32,
    pub v_comb3: i32,
    pub v_comb4: i32,

    pub v_wall: i32,

    pub v_apf1: i32,
    pub v_apf2: i32,

    pub m_lsame: usize,
    pub m_rsame: usize,
    pub d_lsame: usize,
    pub d_rsame: usize,

    pub m_lcomb1: usize,
    pub m_rcomb1: usize,
    pub m_lcomb2: usize,
    pub m_rcomb2: usize,
    pub m_lcomb3: usize,
    pub m_rcomb3: usize,
    pub m_lcomb4: usize,
    pub m_rcomb4: usize,

    pub m_ldiff: usize,
    pub m_rdiff: usize,
    pub d_ldiff: usize,
    pub d_rdiff: usize,

    pub m_lapf1: usize,
    pub m_rapf1: usize,
    pub m_lapf2: usize,
    pub m_rapf2: usize,

    pub half_tick: bool,
    pub current_buffer_addr: usize,

    pub l_out: i32,
    pub r_out: i32,
}

impl Reverb {
    pub fn set_base_addr(&mut self, addr: u16) {
        self.m_base = usize::from(addr) * 8;
        self.current_buffer_addr = self.m_base;
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

        let l_in = mixed[0].saturating_mul(i32::from(self.v_out.l)) >> 15;
        let r_in = mixed[1].saturating_mul(i32::from(self.v_out.l)) >> 15;

        // ____Same Side Reflection (left-to-left and right-to-right)___________________
        // [mLSAME] = (Lin + [dLSAME]*vWALL - [mLSAME-2])*vIIR + [mLSAME-2]  ;L-to-L
        // [mRSAME] = (Rin + [dRSAME]*vWALL - [mRSAME-2])*vIIR + [mRSAME-2]  ;R-to-R

        let ltol = mul_16(
            l_in + mul_16(self.read_sample(ram, self.d_lsame), self.v_wall)
                - self.read_sample(ram, self.m_lsame.saturating_sub(2)),
            self.v_iir,
        ) + self.read_sample(ram, self.m_lsame.saturating_sub(2));

        let rtor = mul_16(
            r_in + mul_16(self.read_sample(ram, self.d_rsame), self.v_wall)
                - self.read_sample(ram, self.m_rsame.saturating_sub(2)),
            self.v_iir,
        ) + self.read_sample(ram, self.m_rsame.saturating_sub(2));

        if write_to_ram {
            self.write_sample(ram, self.m_lsame, ltol);
            self.write_sample(ram, self.m_rsame, rtor);
        }

        // ___Different Side Reflection (left-to-right and right-to-left)_______________
        // [mLDIFF] = (Lin + [dRDIFF]*vWALL - [mLDIFF-2])*vIIR + [mLDIFF-2]  ;R-to-L
        // [mRDIFF] = (Rin + [dLDIFF]*vWALL - [mRDIFF-2])*vIIR + [mRDIFF-2]  ;L-to-R

        let rtol = mul_16(
            l_in + mul_16(self.read_sample(ram, self.d_rdiff), self.v_wall)
                - self.read_sample(ram, self.m_ldiff.saturating_sub(2)),
            self.v_iir,
        ) + self.read_sample(ram, self.m_ldiff.saturating_sub(2));

        let ltor = mul_16(
            r_in + mul_16(self.read_sample(ram, self.d_rdiff), self.v_wall)
                - self.read_sample(ram, self.m_ldiff.saturating_sub(2)),
            self.v_iir,
        ) + self.read_sample(ram, self.m_ldiff.saturating_sub(2));

        if write_to_ram {
            self.write_sample(ram, self.m_lsame, rtol);
            self.write_sample(ram, self.m_rsame, ltor);
        }

        // ___Early Echo (Comb Filter, with input from buffer)__________________________
        // Lout=vCOMB1*[mLCOMB1]+vCOMB2*[mLCOMB2]+vCOMB3*[mLCOMB3]+vCOMB4*[mLCOMB4]
        // Rout=vCOMB1*[mRCOMB1]+vCOMB2*[mRCOMB2]+vCOMB3*[mRCOMB3]+vCOMB4*[mRCOMB4]

        let mut l_out = mul_16(self.v_comb1, self.read_sample(ram, self.m_lcomb1))
            + mul_16(self.v_comb2, self.read_sample(ram, self.m_lcomb2))
            + mul_16(self.v_comb3, self.read_sample(ram, self.m_lcomb3))
            + mul_16(self.v_comb4, self.read_sample(ram, self.m_lcomb4));

        let mut r_out = mul_16(self.v_comb1, self.read_sample(ram, self.m_rcomb1))
            + mul_16(self.v_comb2, self.read_sample(ram, self.m_rcomb2))
            + mul_16(self.v_comb3, self.read_sample(ram, self.m_rcomb3))
            + mul_16(self.v_comb4, self.read_sample(ram, self.m_rcomb4));

        // ___Late Reverb APF1 (All Pass Filter 1, with input from COMB)________________
        // Lout=Lout-vAPF1*[mLAPF1-dAPF1], [mLAPF1]=Lout, Lout=Lout*vAPF1+[mLAPF1-dAPF1]
        // Rout=Rout-vAPF1*[mRAPF1-dAPF1], [mRAPF1]=Rout, Rout=Rout*vAPF1+[mRAPF1-dAPF1]

        l_out -= mul_16(
            self.v_apf1,
            self.read_sample(ram, self.m_lapf1 - self.d_apf1),
        );

        r_out -= mul_16(
            self.v_apf1,
            self.read_sample(ram, self.m_rapf1 - self.d_apf1),
        );

        if write_to_ram {
            self.write_sample(ram, self.m_lapf1, l_out);
            self.write_sample(ram, self.m_lapf1, r_out);
        }

        l_out = mul_16(l_out, self.v_apf1) + self.read_sample(ram, self.m_lapf1 - self.d_apf1);
        r_out = mul_16(r_out, self.v_apf1) + self.read_sample(ram, self.m_rapf1 - self.d_apf1);

        //   ___Late Reverb APF2 (All Pass Filter 2, with input from APF1)________________
        // Lout=Lout-vAPF2*[mLAPF2-dAPF2], [mLAPF2]=Lout, Lout=Lout*vAPF2+[mLAPF2-dAPF2]
        // Rout=Rout-vAPF2*[mRAPF2-dAPF2], [mRAPF2]=Rout, Rout=Rout*vAPF2+[mRAPF2-dAPF2]

        l_out -= mul_16(
            self.v_apf1,
            self.read_sample(ram, self.m_lapf2 - self.d_apf2),
        );

        r_out -= mul_16(
            self.v_apf1,
            self.read_sample(ram, self.m_rapf2 - self.d_apf2),
        );

        if write_to_ram {
            self.write_sample(ram, self.m_lapf2, l_out);
            self.write_sample(ram, self.m_lapf2, r_out);
        }

        l_out = mul_16(l_out, self.v_apf2) + self.read_sample(ram, self.m_lapf2 - self.d_apf2);
        r_out = mul_16(r_out, self.v_apf2) + self.read_sample(ram, self.m_rapf2 - self.d_apf2);

        // ___Output to Mixer (Output volume multiplied with input from APF2)___________
        // LeftOutput  = Lout*vLOUT
        // RightOutput = Rout*vROUT

        self.l_out = mul_16(l_out, i32::from(self.v_out.l));
        self.r_out = mul_16(r_out, i32::from(self.v_out.r));

        // BufferAddress = MAX(mBASE, (BufferAddress+2) AND 7FFFEh)
        self.current_buffer_addr = ((self.current_buffer_addr + 2) & 0x7FFFE).max(self.m_base);
    }

    /// All memory addresses are relative to `current_buffer_addr`,
    /// and wrapped within `base_addr`..=0x7FFFE when exceeding that region.
    fn read_sample(&self, ram: &SoundRam, pos: usize) -> i32 {
        let base = self.m_base;
        let offs = (pos + self.current_buffer_addr - base) % (0x80000 - base);
        let addr = (base + offs) & 0x7FFFE;

        let bytes = [ram[addr], ram[addr + 1]];
        i32::from(i16::from_le_bytes(bytes))
    }

    fn write_sample(&self, ram: &mut SoundRam, pos: usize, val: i32) {
        let base = self.m_base;
        let offs = (pos + self.current_buffer_addr - base) % (0x80000 - base);
        let addr = (base + offs) & 0x7FFFE;

        let bytes = clamped_i16(val).to_le_bytes();
        ram[addr] = bytes[0];
        ram[addr + 1] = bytes[1];
    }
}

/// The multiplication results are divided by +8000h, to fit them to 16bit range.
const fn mul_16(a: i32, b: i32) -> i32 {
    a.saturating_mul(b) >> 15
}
