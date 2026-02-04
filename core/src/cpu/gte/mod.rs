mod commands;
mod mat3;
mod util;
mod vec2;
mod vec3;

use std::ops::{Index, IndexMut};

use tracing::{error, trace};

use mat3::Matrix3;
use vec2::Vector2;
use vec3::Vector3;

use crate::{
    System,
    cpu::utils::{Exception, Instruction},
};

#[derive(Default)]
pub struct GTEngine {
    /// Rotation matrix
    rtm: Matrix3,

    /// Light matrix
    llm: Matrix3,

    /// Light color matrix
    lcm: Matrix3,

    /// Translation vector
    tr: Vector3<i32>,

    /// Background color
    bk: Vector3<i32>,

    /// Far color
    fc: Vector3<i32>,

    /// Screen offset
    of: Vector2<i32>,

    /// Projection plane distance
    h: i16,

    /// Depth queuing parameter A (coeff)
    dqa: i16,

    /// Depth queuing parameter B (offset)
    dqb: i32,

    /// Z3 average scale factor
    zsf3: i16,

    /// Z4 average scale factor
    zsf4: i16,

    /// Average Z value (for Ordering Table)
    otz: u16,

    sxy: FixedFifo<Vector2<i16>, 3>,

    sz: FixedFifo<u16, 4>,

    /// Color and code register
    rgbc: Color,

    /// Prohibited, should not be used
    res1: Color,

    colors: FixedFifo<Color, 3>,

    /// 16-bit vectors
    v: [Vector3<i16>; 3],

    ir: Vector3<i16>,

    /// Interpolation Factor
    ir0: i16,

    // Sum of products values
    mac0: i64,
    macv: Vector3<i64>,

    /// Leading count bit source data
    lzcs: i32,

    /// Returns any calculation errors
    flag: Flag,
}

pub fn cop2(system: &mut System, instr: Instruction) -> Result<(), Exception> {
    check_valid_gte_access(system)?;

    if instr.is_gte_command() {
        system.cpu.gte.command(GteCommand(instr.0));
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
            0 => self.v[0].write_xy(data),
            1 => self.v[0].z = data as i16,

            2 => self.v[1].write_xy(data),
            3 => self.v[1].z = data as i16,

            4 => self.v[2].write_xy(data),
            5 => self.v[2].z = data as i16,

            6 => self.rgbc.write_u32(data),
            7 => self.otz = data as u16,
            8 => self.ir0 = data as i16,

            9 => self.ir.x = data as i16,
            10 => self.ir.y = data as i16,
            11 => self.ir.z = data as i16,

            12..=14 => self.sxy[r - 12].write_u32(data),
            // SXYP is a SXY2 mirror with move-on-write
            15 => self.sxy.push(Vector2::from_u32(data)),

            16..=19 => self.sz[r - 16] = data as u16,

            20..=22 => self.colors[r - 20].write_u32(data),

            // RES1 prohibited/unused but readable and writeable
            23 => self.res1.write_u32(data),

            24 => self.mac0 = data as i64,

            25 => self.macv.x = data as i64,
            26 => self.macv.y = data as i64,
            27 => self.macv.z = data as i64,

            // IRGB and ORGB both are mirrors
            28 => self.set_irgb(ColorConversion(data)),

            // ORGB is read only
            29 => {}

            30 => self.lzcs = data as i32,

            // LZCR is read only
            31 => {}

            32..=36 => self.rtm.write_reg_u32(r - 32, data),

            37 => self.tr.x = data as i32,
            38 => self.tr.y = data as i32,
            39 => self.tr.z = data as i32,

            40..=44 => self.llm.write_reg_u32(r - 40, data),

            45 => self.bk.x = data as i32,
            46 => self.bk.y = data as i32,
            47 => self.bk.z = data as i32,

            48..=52 => self.lcm.write_reg_u32(r - 48, data),

            53 => self.fc.x = data as i32,
            54 => self.fc.y = data as i32,
            55 => self.fc.z = data as i32,

            56 => self.of.x = data as i32,
            57 => self.of.y = data as i32,

            58 => self.h = data as i16,
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
            0 => self.v[0].xy(),
            1 => self.v[0].zs(),

            2 => self.v[1].xy(),
            3 => self.v[1].zs(),

            4 => self.v[2].xy(),
            5 => self.v[2].zs(),

            6 => self.rgbc.as_u32(),
            7 => self.otz as u32,
            8 => self.ir0 as u32,

            9 => self.ir.x as u32,
            10 => self.ir.y as u32,
            11 => self.ir.z as u32,

            12..=14 => self.sxy[r - 12].as_u32(),

            // SXYP is a SXY2 mirror with move-on-write
            15 => self.sxy[2].as_u32(),

            16..=19 => self.sz[r - 16] as u32,

            20..=22 => self.colors[r - 20].as_u32(),

            // RES1 prohibited/unused but readable and writeable
            23 => self.res1.as_u32(),

            24 => self.mac0 as u32,

            25 => self.macv.x as u32,
            26 => self.macv.y as u32,
            27 => self.macv.z as u32,

            // IRGB and ORGB both are mirrors
            28..=29 => self.orgb(),

            30 => self.lzcs as u32,
            31 => self.lzcr(),

            32..=36 => self.rtm.as_reg_u32(r - 32),

            37 => self.tr.x as u32,
            38 => self.tr.y as u32,
            39 => self.tr.z as u32,

            40..=44 => self.llm.as_reg_u32(r - 40),

            45 => self.bk.x as u32,
            46 => self.bk.y as u32,
            47 => self.bk.z as u32,

            48..=52 => self.lcm.as_reg_u32(r - 48),

            53 => self.fc.x as u32,
            54 => self.fc.y as u32,
            55 => self.fc.z as u32,

            56 => self.of.x as u32,
            57 => self.of.y as u32,

            58 => self.h as u32,
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
        self.ir.x = irgb.r() as i16 * 0x80;
        self.ir.y = irgb.g() as i16 * 0x80;
        self.ir.z = irgb.b() as i16 * 0x80;
    }

    /// Output color conversion register (mirror of irgb)
    fn orgb(&self) -> u32 {
        let mut orgb = ColorConversion(0);
        orgb.set_r((self.ir.x / 0x80).clamp(0, 0x1F) as u16);
        orgb.set_g((self.ir.y / 0x80).clamp(0, 0x1F) as u16);
        orgb.set_b((self.ir.z / 0x80).clamp(0, 0x1F) as u16);
        orgb.0
    }

    fn command(&mut self, cmd: GteCommand) {
        self.flag.clear();

        match cmd.opcode() {
            0x01 => self.rtps(),
            0x06 => self.nclip(),
            0x0C => self.op(cmd),
            0x10 => self.dpcs(),
            0x11 => self.intpl(),
            0x12 => self.mvmva(),
            0x13 => self.ncds(),
            0x14 => self.cdp(),
            0x16 => self.ncdt(),
            0x1B => self.nccs(),
            0x1C => self.cc(),
            0x1E => self.ncs(),
            0x20 => self.nct(),
            0x28 => self.sqr(),
            0x29 => self.dcpl(),
            0x2A => self.dpct(),
            0x2D => self.avsz3(),
            0x2E => self.avsz4(),
            0x30 => self.rtpt(),
            0x3D => self.gpf(),
            0x3E => self.gpl(),
            0x3F => self.ncct(),
            x => unimplemented!("GTE command {x:x}"),
        }
    }

    // fn unr_div(&self) -> Option<i64> {
    //     let h = self.h as u32;
    //     let sz3 = self.sz.fifo[3];
    //
    //     if h >= (sz3 as u32 * 2) {
    //         return None;
    //     }
    //
    //     let z = sz3.leading_zeros();
    //     let n = h << z;
    //     let d_norm = (sz3 << z) as i64;
    //
    //     let u = UNR_TABLE[((d_norm - 0x7FC0) >> 7) as usize] + 0x101;
    //
    //     let d_refine = (0x2000080 - (d_norm * u)) >> 8;
    //     let d_final = (0x0000080 + (d_refine * u)) >> 8;
    //
    //     let res = ((n as i64 * d_final) + 0x8000) >> 16;
    //
    //     Some(0x1FFFF.min(res))
    // }
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
    pub struct GteCommand(u32);
    u8, sf, _: 19, 19;
    u8, into MMVAMultiplyMatrix, mx, _: 18, 17;
    u8, into MMVAMultiplyVector, vx, _: 16, 15;
    u8, into MMVATranslationVector, tx, _: 14, 13;
    u8, into SaturationRange, lm, _: 10, 10;
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

#[derive(Default, Clone, Copy)]
struct Color {
    c: u8,
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    fn as_u32(&self) -> u32 {
        u32::from_le_bytes([self.c, self.b, self.g, self.r])
    }

    fn write_u32(&mut self, v: u32) {
        let bytes = v.to_le_bytes();
        self.c = bytes[0];
        self.b = bytes[1];
        self.g = bytes[2];
        self.r = bytes[3];
    }
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
            fifo: std::array::from_fn(|_| T::default()),
        }
    }
}

enum MMVAMultiplyMatrix {
    Rotation,
    Light,
    Color,
    Reserved,
}

impl From<u8> for MMVAMultiplyMatrix {
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

enum MMVAMultiplyVector {
    V0,
    V1,
    V2,
    IR,
}

impl From<u8> for MMVAMultiplyVector {
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

enum MMVATranslationVector {
    TranslationVector,
    BackgroundColor,
    FarColor,
    None,
}

impl From<u8> for MMVATranslationVector {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::TranslationVector,
            1 => Self::BackgroundColor,
            2 => Self::FarColor,
            3 => Self::None,
            _ => panic!("2 bit cannot reach here"),
        }
    }
}

enum SaturationRange {
    Unsigned15,
    Signed16,
}

impl From<u8> for SaturationRange {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Signed16,
            1 => Self::Unsigned15,
            _ => unreachable!("bool cannot reach here"),
        }
    }
}
