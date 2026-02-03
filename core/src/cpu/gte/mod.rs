mod commands;
mod math;

use tracing::{debug, error};

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

    sxy: ScreenXYFifo,

    sz: ScreenZFifo,

    /// Color and code register
    rgbc: Color,

    /// Prohibited, should not be used
    res1: Color,

    colors: ColorFifo,

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
        _ => unimplemented!(
            "GTE instruction instr={:#08x} pc={:08x}",
            instr.0,
            system.cpu.pc
        ),
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

            12..=14 => self.sxy.fifo[r - 12].write_u32(data),
            // SXYP is a SXY2 mirror with move-on-write
            15 => self.sxy.push(Vector2::from_u32(data)),

            16..=19 => self.sz.fifo[r - 16] = data as u16,

            20..=22 => self.colors.fifo[r - 20].write_u32(data),

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

            12..=14 => self.sxy.fifo[r - 12].as_u32(),

            // SXYP is a SXY2 mirror with move-on-write
            15 => self.sxy.fifo[2].as_u32(),

            16..=19 => self.sz.fifo[r - 16] as u32,

            20..=22 => self.colors.fifo[r - 20].as_u32(),

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
            0x01 => self.rtps(cmd),
            0x06 => self.nclip(),
            0x13 => self.ncds(),
            0x30 => self.rtpt(),
            x => unimplemented!("GTE command {x:x}"),
        }
    }
}

/// Transfer from data register
fn mfc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd();

    let data = system.cpu.gte.read_reg(cop_r);
    debug!("mfc2: cpu_reg{cpu_r} write <- cop2r{cop_r} = {data:x}");

    system.cpu.take_delayed_load(cpu_r, data);
}

/// Transfer from control register
fn cfc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd() + 32;

    let data = system.cpu.gte.read_reg(cop_r);
    debug!("cfc2: cpu_reg{cpu_r} write <- cop2r{cop_r} = {data:x}");

    system.cpu.take_delayed_load(cpu_r, data);
}

/// Transfer to data register
fn mtc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd();

    let data = system.cpu.regs[cpu_r];
    debug!("mtc2: cop2r{cop_r} write <- {data:x}");

    system.cpu.gte.write_reg(cop_r, data);
}

/// Transfer to control register
fn ctc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd() + 32;

    let data = system.cpu.regs[cpu_r];
    debug!("ctc2: cop2r{} write <- {data:x}", cop_r);

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

    debug!("lwc2: cop2r{rt} <- {data:x}");

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

    debug!("swc2: {addr:08x} <- cop2r{rt}");

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

#[derive(Default)]
struct ColorFifo {
    fifo: [Color; 3],
}

impl ColorFifo {
    fn push(&mut self, v: Color) {
        self.fifo[0] = self.fifo[1];
        self.fifo[1] = self.fifo[2];
        self.fifo[2] = v;
    }
}

#[derive(Default, Debug)]
struct Matrix3 {
    elems: [i16; 9],
}

impl Matrix3 {
    fn write_reg_u32(&mut self, r: usize, v: u32) {
        if r == 4 {
            self.elems[8] = (v & 0xFFFF) as i16;
            return;
        }

        self.elems[r * 2 + 1] = (v >> 16) as i16;
        self.elems[r * 2] = (v & 0xFFFF) as i16;
    }

    fn as_reg_u32(&self, r: usize) -> u32 {
        if r == 4 {
            return self.elems[8] as u32;
        }

        let msb = self.elems[r * 2 + 1] as u32;
        let lsb = self.elems[r * 2] as u32;

        (msb << 16) | (lsb & 0xFFFF)
    }
}

#[derive(Default)]
struct ScreenXYFifo {
    fifo: [Vector2<i16>; 3],
}

impl ScreenXYFifo {
    fn push(&mut self, v: Vector2<i16>) {
        self.fifo[0] = self.fifo[1];
        self.fifo[1] = self.fifo[2];
        self.fifo[2] = v;
    }
}

#[derive(Default)]
struct ScreenZFifo {
    fifo: [u16; 4],
}

impl ScreenZFifo {
    fn push(&mut self, v: u16) {
        self.fifo[0] = self.fifo[1];
        self.fifo[1] = self.fifo[2];
        self.fifo[2] = self.fifo[3];
        self.fifo[3] = v;
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct Vector3<T> {
    x: T,
    y: T,
    z: T,
}

#[derive(Default, Debug, Clone, Copy)]
struct Vector2<T> {
    x: T,
    y: T,
}

impl Vector2<i16> {
    fn from_u32(v: u32) -> Self {
        Self {
            y: (v >> 16) as i16,
            x: (v & 0xFFFF) as i16,
        }
    }

    fn write_u32(&mut self, v: u32) {
        *self = Self::from_u32(v);
    }

    fn as_u32(&self) -> u32 {
        (self.y as u32) << 16 | (self.x as u32) & 0xFFFF
    }
}

impl Vector3<i16> {
    fn write_xy(&mut self, v: u32) {
        self.y = (v >> 16) as i16;
        self.x = (v & 0xFFFF) as i16;
    }

    fn xy(&self) -> u32 {
        (self.y as u32) << 16 | self.x as u32 & 0xFFFF
    }

    /// Sign extended z value
    fn zs(&self) -> u32 {
        self.z as u32
    }
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
