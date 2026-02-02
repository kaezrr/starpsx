mod commands;

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

    tr: TranslationVector,

    bk: BackgroundColor,

    fc: FarColor,

    of: ScreenOffset,

    /// Projection plane distance
    h: FixedI16<0>,

    /// Depth queuing parameter A (coeff)
    dqa: FixedI16<8>,

    /// Depth queuing parameter B (offset)
    dqb: FixedI32<24>,

    /// Z3 average scale factor
    zsf3: FixedI16<12>,

    /// Z4 average scale factor
    zsf4: FixedI16<12>,

    /// Average Z value (for Ordering Table)
    otz: FixedU16<0>,

    sxy: ScreenXYFifo,

    sz: ScreenZFifo,

    rgbc: Color,

    colors: ColorFifo,

    /// 16-bit vectors
    v: [Vector3; 3],

    ir: InterpolationVector,

    /// Interpolation Factor
    ir0: FixedU16<12>,

    // Sum of products values
    mac: [FixedI32<0>; 4],

    /// Leading count bit source data
    lzcs: FixedI32<0>,

    /// Returns any calculation errors
    flag: Flag,
}

pub fn cop2(system: &mut System, instr: Instruction) -> Result<(), Exception> {
    check_valid_gte_access(system)?;

    if instr.is_gte_command() {
        system.cpu.gte.command(instr);
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
            1 => self.v[0].z.write_u32(data),

            2 => self.v[1].write_xy(data),
            3 => self.v[1].z.write_u32(data),

            4 => self.v[2].write_xy(data),
            5 => self.v[2].z.write_u32(data),

            6 => self.rgbc.write_u32(data),
            7 => self.otz.write_u32(data),
            8 => self.ir0.write_u32(data),

            9 => self.ir.ir1.write_u32(data),
            10 => self.ir.ir2.write_u32(data),
            11 => self.ir.ir3.write_u32(data),

            12..=14 => self.sxy.fifo[r - 12].write_u32(data),
            // SXYP is a SXY2 mirror with move-on-write
            15 => self.sxy.push(Vector2::from_u32(data)),

            16..=19 => self.sz.fifo[r - 16].write_u32(data),

            20..=22 => self.colors.fifo[r - 20].write_u32(data),

            24..=27 => self.mac[r - 24].write_u32(data),

            // IRGB and ORGB both are mirrors
            28 => self.ir.set_irgb(ColorConversion(data)),

            30 => self.lzcs.write_u32(data),

            32..=36 => self.rtm.write_reg_u32(r - 32, data),

            37 => self.tr.x.write_u32(data),
            38 => self.tr.y.write_u32(data),
            39 => self.tr.z.write_u32(data),

            40..=44 => self.llm.write_reg_u32(r - 40, data),

            45 => self.bk.r.write_u32(data),
            46 => self.bk.g.write_u32(data),
            47 => self.bk.b.write_u32(data),

            48..=52 => self.lcm.write_reg_u32(r - 48, data),

            53 => self.fc.r.write_u32(data),
            54 => self.fc.g.write_u32(data),
            55 => self.fc.b.write_u32(data),

            56 => self.of.x.write_u32(data),
            57 => self.of.y.write_u32(data),

            58 => self.h.write_u32(data),
            59 => self.dqa.write_u32(data),
            60 => self.dqb.write_u32(data),

            61 => self.zsf3.write_u32(data),
            62 => self.zsf4.write_u32(data),

            // Bits 30 - 12 writable
            63 => self.flag.0 |= data & 0x7FFFF000,

            _ => unimplemented!("invalid GTE register read: {r}"),
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
            7 => self.otz.as_u32(),
            8 => self.ir0.as_u32(),

            9 => self.ir.ir1.as_u32(),
            10 => self.ir.ir2.as_u32(),
            11 => self.ir.ir3.as_u32(),

            12..=14 => self.sxy.fifo[r - 12].as_u32(),

            // SXYP is a SXY2 mirror with move-on-write
            15 => self.sxy.fifo[2].as_u32(),

            16..=19 => self.sz.fifo[r - 16].as_u32(),

            20..=22 => self.colors.fifo[r - 20].as_u32(),

            24..=27 => self.mac[r - 24].as_u32(),

            // IRGB and ORGB both are mirrors
            28..=29 => self.ir.as_orgb().0,

            30 => self.lzcs.as_u32(),
            31 => self.lzcr(),

            32..=36 => self.rtm.as_reg_u32(r - 32),

            37 => self.tr.x.as_u32(),
            38 => self.tr.y.as_u32(),
            39 => self.tr.z.as_u32(),

            40..=44 => self.llm.as_reg_u32(r - 40),

            45 => self.bk.r.as_u32(),
            46 => self.bk.g.as_u32(),
            47 => self.bk.b.as_u32(),

            48..=52 => self.lcm.as_reg_u32(r - 48),

            53 => self.fc.r.as_u32(),
            54 => self.fc.g.as_u32(),
            55 => self.fc.b.as_u32(),

            56 => self.of.x.as_u32(),
            57 => self.of.y.as_u32(),

            58 => self.h.as_u32(),
            59 => self.dqa.as_u32(),
            60 => self.dqb.as_u32(),

            61 => self.zsf3.as_u32(),
            62 => self.zsf4.as_u32(),

            63 => self.flag.read(),

            _ => unimplemented!("invalid GTE register read: {r}"),
        }
    }

    /// Counting leading bits result
    fn lzcr(&self) -> u32 {
        if self.lzcs.0.is_negative() {
            self.lzcs.0.leading_ones()
        } else {
            self.lzcs.0.leading_zeros()
        }
    }

    fn command(&mut self, instr: Instruction) {
        match instr.sec() {
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
    debug!("mfc2: cpu_reg{cpu_r} write <- {data:x}");

    system.cpu.take_delayed_load(cpu_r, data);
}

/// Transfer from control register
fn cfc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd() + 32;

    let data = system.cpu.gte.read_reg(cop_r);
    debug!("cfc2: cpu_reg{cpu_r} write <- {data:x}");

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
    debug!("ctc2: cop2r{} write <- {data:x}", cop_r + 32);

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
}

bitfield::bitfield! {
#[derive(Default)]
    struct ColorConversion(u32);
    i16, r, set_r: 4, 0;
    i16, g, set_g: 9, 5;
    i16, b, set_b: 14, 10;
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

#[derive(Default)]
struct TranslationVector {
    x: FixedI32<0>,
    y: FixedI32<0>,
    z: FixedI32<0>,
}

#[derive(Default)]
struct BackgroundColor {
    r: FixedI32<12>,
    g: FixedI32<12>,
    b: FixedI32<12>,
}

#[derive(Default)]
struct FarColor {
    r: FixedI32<4>,
    g: FixedI32<4>,
    b: FixedI32<4>,
}

#[derive(Default)]
struct ScreenOffset {
    x: FixedI16<16>,
    y: FixedI16<16>,
}

#[derive(Default)]
struct Matrix3 {
    elems: [FixedI16<12>; 9],
}

impl Matrix3 {
    fn write_reg_u32(&mut self, r: usize, v: u32) {
        if r == 4 {
            self.elems[8] = ((v & 0xFFFF) as i16).into();
            return;
        }

        self.elems[r * 2 + 1] = ((v >> 16) as i16).into();
        self.elems[r * 2] = ((v & 0xFFFF) as i16).into();
    }

    fn as_reg_u32(&self, r: usize) -> u32 {
        if r == 4 {
            return self.elems[8].as_u32();
        }

        let msb = self.elems[r * 2 + 1].as_u32();
        let lsb = self.elems[r * 2].as_u32();

        (msb << 16) | lsb
    }
}

#[derive(Default)]
struct ScreenXYFifo {
    fifo: [Vector2; 3],
}

impl ScreenXYFifo {
    fn push(&mut self, v: Vector2) {
        self.fifo[0] = self.fifo[1];
        self.fifo[1] = self.fifo[2];
        self.fifo[2] = v;
    }
}

#[derive(Default)]
struct ScreenZFifo {
    fifo: [FixedU16<0>; 4],
}

impl ScreenZFifo {
    fn push(&mut self, v: FixedU16<0>) {
        self.fifo[0] = self.fifo[1];
        self.fifo[1] = self.fifo[2];
        self.fifo[2] = self.fifo[3];
        self.fifo[3] = v;
    }
}

#[derive(Default, Clone, Copy)]
struct Vector3 {
    x: FixedI16<0>,
    y: FixedI16<0>,
    z: FixedI16<0>,
}

#[derive(Default, Clone, Copy)]
struct Vector2 {
    x: FixedI16<0>,
    y: FixedI16<0>,
}

impl Vector2 {
    fn from_u32(v: u32) -> Self {
        let msb = (v >> 16) as i16;
        let lsb = (v & 0xFFFF) as i16;
        Self {
            y: msb.into(),
            x: lsb.into(),
        }
    }

    fn write_u32(&mut self, v: u32) {
        *self = Self::from_u32(v);
    }

    fn as_u32(&self) -> u32 {
        (self.y.as_u32() << 16) | self.x.as_u32()
    }
}

impl Vector3 {
    fn write_xy(&mut self, v: u32) {
        let msb = (v >> 16) as i16;
        let lsb = (v & 0xFFFF) as i16;
        self.y = msb.into();
        self.x = lsb.into();
    }

    fn xy(&self) -> u32 {
        (self.y.as_u32() << 16) | self.x.as_u32()
    }

    /// Sign extended z value
    fn zs(&self) -> u32 {
        self.z.as_u32()
    }
}

#[derive(Default)]
struct InterpolationVector {
    ir1: FixedI16<0>,
    ir2: FixedI16<0>,
    ir3: FixedI16<0>,
}

impl InterpolationVector {
    fn set_irgb(&mut self, irgb: ColorConversion) {
        self.ir1 = (irgb.r() * 0x80).into();
        self.ir2 = (irgb.g() * 0x80).into();
        self.ir3 = (irgb.b() * 0x80).into();
    }

    fn as_orgb(&mut self) -> ColorConversion {
        let mut orgb = ColorConversion(0);
        orgb.set_r((self.ir1.0 / 0x80).clamp(0, 0x1F));
        orgb.set_g((self.ir2.0 / 0x80).clamp(0, 0x1F));
        orgb.set_b((self.ir3.0 / 0x80).clamp(0, 0x1F));
        orgb
    }
}

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
struct Fixed<T, const FB: usize>(T);

type FixedU16<const FB: usize> = Fixed<u16, FB>;
type FixedI16<const FB: usize> = Fixed<i16, FB>;
type FixedI32<const FB: usize> = Fixed<i32, FB>;

impl<T, const FB: usize> From<T> for Fixed<T, FB> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

macro_rules! fixed_lossy_as_u32 {
    ($($t:ty),+) => {
        $(
            impl<const FB: usize> Fixed<$t, FB> {
                #[inline]
                fn as_u32(&self) -> u32 {
                    self.0 as u32
                }
            }

            impl<const FB: usize> Fixed<$t, FB> {
                #[inline]
                fn write_u32(&mut self, v: u32) {
                    self.0 = v as $t;
                }
            }
        )+
    };
}

fixed_lossy_as_u32!(u16, i16, i32);
