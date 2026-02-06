mod commands;
mod utils;

use std::ops::{Index, IndexMut};

use tracing::{error, trace};

use crate::{
    System,
    cpu::utils::{Exception, Instruction},
};

use utils::{matrix_reg_read, matrix_reg_write, vec_xy_read, vec_xy_write};

#[derive(Default)]
pub struct GTEngine {
    /// Rotation, light and light color matrices (1, 3, 12)
    /// Last one is a garbage matrix
    matrices: [[[i16; 3]; 3]; 4],

    /// Translation vector (1, 31, 0)
    /// Background color (1, 19, 12)
    /// Far color (1, 27, 4)
    /// Empty/None vector
    control_vectors: [[i32; 3]; 4],

    /// Screen offset (1, 15, 16)
    of: [i32; 2],

    /// Projection plane distance (0, 16, 0)
    h: u16,

    /// Depth queuing parameter A (coeff) (1, 7, 8)
    dqa: i16,

    /// Depth queuing parameter B (offset) (1, 7, 24)
    dqb: i32,

    /// Z3 average scale factor (1, 3, 12)
    zsf3: i16,

    /// Z4 average scale factor (1, 3, 12)
    zsf4: i16,

    /// Average Z value (for Ordering Table) (0, 15, 0)
    otz: u16,

    /// Screen XY coordinates (1, 15, 0)
    sxy: FixedFifo<[i16; 2], 3>,

    /// Screen Z coordinates (0, 16, 0)
    sz: FixedFifo<u16, 4>,

    /// Color and code register (0, 8, 0) Red Green Blue Code
    rgbc: [u8; 4],

    /// Prohibited, should not be used
    res1: [u8; 4],

    /// Color and code registers fifo (0, 8, 0) Red Green Blue Code
    colors: FixedFifo<[u8; 4], 3>,

    /// 16-bit vectors (1, 3, 12) or (1, 15, 0)
    v: [[i16; 3]; 4],

    /// Interpolation Factors (1, 3, 12)
    ir: [i16; 4],

    // Sum of products values (1, 31, 0)
    mac: [i32; 4],

    /// Leading count bit source data (1, 31, 0)
    lzcs: i32,

    /// Returns any calculation errors
    flag: Flag,
}

pub fn cop2(system: &mut System, instr: Instruction) -> Result<(), Exception> {
    check_valid_gte_access(system)?;

    if instr.is_gte_command() {
        system.cpu.gte.command(CommandFields(instr.0));
        return Ok(());
    }

    match instr.rs() {
        0x00 => mfc2(system, instr),
        0x02 => cfc2(system, instr),
        0x04 => mtc2(system, instr),
        0x06 => ctc2(system, instr),
        _ => unimplemented!("GTE instruction instr={:#08x} ", instr.0),
    };

    Ok(())
}

impl GTEngine {
    fn write_reg(&mut self, r: usize, data: u32) {
        match r {
            0 => vec_xy_write(&mut self.v[0], data),
            1 => self.v[0][2] = data as i16,

            2 => vec_xy_write(&mut self.v[1], data),
            3 => self.v[1][2] = data as i16,

            4 => vec_xy_write(&mut self.v[2], data),
            5 => self.v[2][2] = data as i16,

            6 => self.rgbc = data.to_le_bytes(),
            7 => self.otz = data as u16,

            8..=11 => self.ir[r - 8] = data as i16,

            12..=14 => vec_xy_write(&mut self.sxy[r - 12], data),
            // SXYP is a SXY2 mirror with move-on-write
            15 => {
                let mut v = [0; 2];
                vec_xy_write(&mut v, data);
                self.sxy.push(v);
            }

            16..=19 => self.sz[r - 16] = data as u16,

            20..=22 => self.colors[r - 20] = data.to_le_bytes(),

            // RES1 prohibited/unused but readable and writeable
            23 => self.res1 = data.to_le_bytes(),

            24..=27 => self.mac[r - 24] = data as i32,

            // IRGB and ORGB both are mirrors
            28 => self.set_irgb(ColorConversion(data)),

            // ORGB is read only
            29 => {}

            30 => self.lzcs = data as i32,

            // LZCR is read only
            31 => {}

            32..=36 => matrix_reg_write(&mut self.matrices[0], r - 32, data),

            37..=39 => self.control_vectors[0][r - 37] = data as i32,

            40..=44 => matrix_reg_write(&mut self.matrices[1], r - 40, data),

            45..=47 => self.control_vectors[1][r - 45] = data as i32,

            48..=52 => matrix_reg_write(&mut self.matrices[2], r - 48, data),

            53..=55 => self.control_vectors[2][r - 53] = data as i32,

            56..=57 => self.of[r - 56] = data as i32,

            58 => self.h = data as u16,
            59 => self.dqa = data as i16,
            60 => self.dqb = data as i32,

            61 => self.zsf3 = data as i16,
            62 => self.zsf4 = data as i16,

            63 => self.flag.write(data),

            _ => unimplemented!("invalid GTE register write: {r}"),
        };
    }

    fn read_reg(&mut self, r: usize) -> u32 {
        match r {
            0 => vec_xy_read(&self.v[0]),
            1 => self.v[0][2] as u32,

            2 => vec_xy_read(&self.v[1]),
            3 => self.v[1][2] as u32,

            4 => vec_xy_read(&self.v[2]),
            5 => self.v[2][2] as u32,

            6 => u32::from_le_bytes(self.rgbc),
            7 => self.otz as u32,

            8..=11 => self.ir[r - 8] as u32,

            12..=14 => vec_xy_read(&self.sxy[r - 12]),

            // SXYP is a SXY2 mirror with move-on-write
            15 => vec_xy_read(&self.sxy[2]),

            16..=19 => self.sz[r - 16] as u32,

            20..=22 => u32::from_le_bytes(self.colors[r - 20]),

            // RES1 prohibited/unused but readable and writeable
            23 => u32::from_le_bytes(self.res1),

            24..=27 => self.mac[r - 24] as u32,

            // IRGB and ORGB both are mirrors
            28..=29 => self.orgb(),

            30 => self.lzcs as u32,
            31 => self.lzcr(),

            32..=36 => matrix_reg_read(&self.matrices[0], r - 32),

            37..=39 => self.control_vectors[0][r - 37] as u32,

            40..=44 => matrix_reg_read(&self.matrices[1], r - 40),

            45..=47 => self.control_vectors[1][r - 45] as u32,

            48..=52 => matrix_reg_read(&self.matrices[2], r - 48),

            53..=55 => self.control_vectors[2][r - 53] as u32,

            56..=57 => self.of[r - 56] as u32,

            // This gets sign extended despite being unsigned
            58 => self.h as i16 as u32,
            59 => self.dqa as u32,
            60 => self.dqb as u32,

            61 => self.zsf3 as u32,
            62 => self.zsf4 as u32,

            63 => self.flag.read(),

            _ => unimplemented!("invalid GTE register read: {r}"),
        }
    }

    /// Counting leading bits result
    fn lzcr(&self) -> u32 {
        if self.lzcs.is_negative() {
            self.lzcs.leading_ones()
        } else {
            self.lzcs.leading_zeros()
        }
    }

    fn set_irgb(&mut self, irgb: ColorConversion) {
        self.ir[1] = irgb.r() as i16 * 0x80;
        self.ir[2] = irgb.g() as i16 * 0x80;
        self.ir[3] = irgb.b() as i16 * 0x80;
    }

    /// Output color conversion register (mirror of irgb)
    fn orgb(&self) -> u32 {
        let mut orgb = ColorConversion(0);
        orgb.set_r((self.ir[1] / 0x80).clamp(0, 0x1F) as u16);
        orgb.set_g((self.ir[2] / 0x80).clamp(0, 0x1F) as u16);
        orgb.set_b((self.ir[3] / 0x80).clamp(0, 0x1F) as u16);
        orgb.0
    }

    fn command(&mut self, fields: CommandFields) {
        self.flag.clear();

        match fields.opcode() {
            0x01 => self.rtps(fields),
            0x06 => self.nclip(),
            0x0C => self.op(fields),
            0x10 => self.dpcs(fields),
            0x11 => self.intpl(fields),
            0x12 => self.mvmva(fields),
            0x13 => self.ncds(fields),
            0x14 => self.cdp(fields),
            0x16 => self.ncdt(fields),
            0x1B => self.nccs(fields),
            0x1C => self.cc(fields),
            0x1E => self.ncs(fields),
            0x20 => self.nct(fields),
            0x28 => self.sqr(fields),
            0x29 => self.dcpl(fields),
            0x2A => self.dpct(fields),
            0x2D => self.avsz3(),
            0x2E => self.avsz4(),
            0x30 => self.rtpt(fields),
            0x3D => self.gpf(fields),
            0x3E => self.gpl(fields),
            0x3F => self.ncct(fields),
            x => unimplemented!("GTE command {x:x}"),
        }
    }
}

/// Transfer from data register
fn mfc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd();

    let data = system.cpu.gte.read_reg(cop_r);
    trace!("mfc2: cpu_reg{cpu_r} write <- cop2r{cop_r} = {data:x}");

    system.cpu.take_delayed_load(cpu_r, data);
}

/// Transfer from control register
fn cfc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd() + 32;

    let data = system.cpu.gte.read_reg(cop_r);
    trace!("cfc2: cpu_reg{cpu_r} write <- cop2r{cop_r} = {data:x}");

    system.cpu.take_delayed_load(cpu_r, data);
}

/// Transfer to data register
fn mtc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd();

    let data = system.cpu.regs[cpu_r];
    trace!("mtc2: cop2r{cop_r} write <- {data:x}");

    system.cpu.gte.write_reg(cop_r, data);
}

/// Transfer to control register
fn ctc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd() + 32;

    let data = system.cpu.regs[cpu_r];
    trace!("ctc2: cop2r{} write <- {data:x}", cop_r);

    system.cpu.gte.write_reg(cop_r, data);
}

/// Load GTE data register
pub fn lwc2(system: &mut System, instr: Instruction) -> Result<(), Exception> {
    check_valid_gte_access(system)?;

    let rs = instr.rs();
    let rt = instr.rt();
    let im = instr.imm16_se();

    let addr = system.cpu.regs[rs].wrapping_add(im);
    let data = system.read::<u32>(addr)?;

    trace!("lwc2: cop2r{rt} <- {data:x}");

    // Needs load delay
    system.cpu.gte.write_reg(rt, data);
    Ok(())
}

/// Store GTE data register
pub fn swc2(system: &mut System, instr: Instruction) -> Result<(), Exception> {
    check_valid_gte_access(system)?;

    let rs = instr.rs();
    let rt = instr.rt();
    let im = instr.imm16_se();

    let addr = system.cpu.regs[rs].wrapping_add(im);
    let data = system.cpu.gte.read_reg(rt);

    trace!("swc2: {addr:08x} <- cop2r{rt}");

    system.write::<u32>(addr, data)?;
    Ok(())
}

#[inline(always)]
fn check_valid_gte_access(system: &System) -> Result<(), Exception> {
    if system.cpu.cop0.gte_enabled() {
        return Ok(());
    }
    error!("coprocessor error, trying to access gte while disabled");
    Err(Exception::CoprocessorError)
}

bitfield::bitfield! {
    #[derive(Clone, Copy)]
    pub struct CommandFields(u32);
    u8, sf, _: 19, 19;
    u8, into Matrix, mx, _: 18, 17;
    u8, into Vector, vx, _: 16, 15;
    u8, into ControlVec, cv, _: 14, 13;
    u8, into Saturation, lm, _: 10, 10;
    u8, opcode, _ : 5, 0;
}

bitfield::bitfield! {
#[derive(Default)]
    struct Flag(u32);
    u8, err1, _: 30, 23;
    u8, err2, _: 18, 13;

    _, mac1_overflow_pos: 30;
    _, mac2_overflow_pos: 29;
    _, mac3_overflow_pos: 28;

    _, mac1_overflow_neg: 27;
    _, mac2_overflow_neg: 26;
    _, mac3_overflow_neg: 25;

    _, ir1_saturated: 24;
    _, ir2_saturated: 23;
    _, ir3_saturated: 22;

    _, cfifo_r_saturated: 21;
    _, cfifo_g_saturated: 20;
    _, cfifo_b_saturated: 19;

    _, sz3_or_otz_saturated: 18;

    _, div_overflow: 17;

    _, mac0_overflow_pos: 16;
    _, mac0_overflow_neg: 15;

    _, sx2_saturated: 14;
    _, sy2_saturated: 13;

    _, ir0_saturated: 12;
}

impl Flag {
    fn read(&self) -> u32 {
        let is_err = (self.err1() | self.err2()) != 0;
        self.0 | ((is_err as u32) << 31)
    }

    fn write(&mut self, v: u32) {
        self.0 = v & 0x7FFFF000;
    }

    fn clear(&mut self) {
        self.0 = 0;
    }
}

bitfield::bitfield! {
#[derive(Default)]
    struct ColorConversion(u32);
    u16, r, set_r: 4, 0;
    u16, g, set_g: 9, 5;
    u16, b, set_b: 14, 10;
}

/// Fixed size first-in-first-out data structure
struct FixedFifo<T, const LEN: usize>
where
    T: Default + Copy,
{
    fifo: [T; LEN],
}

impl<T, const LEN: usize> FixedFifo<T, LEN>
where
    T: Copy + Default,
{
    // Shift fifo by LEN units and insert new value at the back
    fn push(&mut self, v: T) {
        for i in 0..(LEN - 1) {
            self.fifo[i] = self.fifo[i + 1];
        }
        self.fifo[LEN - 1] = v;
    }
}

impl<T, const LEN: usize> IndexMut<usize> for FixedFifo<T, LEN>
where
    T: Default + Copy,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        debug_assert!(index < LEN);
        &mut self.fifo[index]
    }
}

impl<T, const LEN: usize> Index<usize> for FixedFifo<T, LEN>
where
    T: Default + Copy,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        debug_assert!(index < LEN);
        &self.fifo[index]
    }
}

impl<T, const LEN: usize> Default for FixedFifo<T, LEN>
where
    T: Default + Copy,
{
    fn default() -> Self {
        FixedFifo {
            fifo: [T::default(); LEN],
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Matrix {
    Rotation,
    Light,
    Color,
    Reserved,
}

impl From<u8> for Matrix {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Rotation,
            1 => Self::Light,
            2 => Self::Color,
            3 => Self::Reserved,
            _ => unreachable!("2-bit cannot reach here"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Vector {
    V0,
    V1,
    V2,
    IR,
}

impl From<u8> for Vector {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::V0,
            1 => Self::V1,
            2 => Self::V2,
            3 => Self::IR,
            _ => unreachable!("2 bit cannot reach here"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ControlVec {
    Translation,
    Background,
    FarColor,
    None,
}

impl From<u8> for ControlVec {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Translation,
            1 => Self::Background,
            2 => Self::FarColor,
            3 => Self::None,
            _ => panic!("2 bit cannot reach here"),
        }
    }
}

#[derive(Clone, Copy)]
enum Saturation {
    S16,
    U15,
}

impl From<u8> for Saturation {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::S16,
            1 => Self::U15,
            _ => unreachable!("bool cannot reach here"),
        }
    }
}
