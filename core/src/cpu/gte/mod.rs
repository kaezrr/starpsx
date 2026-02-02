mod commands;

use arrayvec::ArrayVec;
use tracing::{debug, error};

use crate::{
    System,
    cpu::utils::{Exception, Instruction},
};

#[derive(Default)]
pub struct GTEngine {
    /// Data registers 0 - 31
    data_regs: [u32; 32],

    /// Control registers 32 - 63
    control_regs: [u32; 32],

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

    sxyz: ScreenCoordsFifo,

    v0: Vector,

    v1: Vector,

    v2: Vector,

    /// Interpolation Vector
    ir: (FixedI16<0>, FixedI16<0>, FixedI16<0>),

    /// Interpolation Factor
    ir0: FixedU16<12>,

    // Sum of products values
    mac0: FixedI32<0>,
    mac1: FixedI32<0>,
    mac2: FixedI32<0>,
    mac3: FixedI32<0>,

    /// Leading count bit source data
    lzcs: i32,

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
    /// Counting Leading Bits Result
    fn lzcr(&self) -> u32 {
        if self.lzcs.is_negative() {
            self.lzcs.leading_ones()
        } else {
            self.lzcs.leading_zeros()
        }
    }

    /// Color Conversion Input
    fn write_irgb(&mut self, data: u32) {
        let irgb = ColorConversionInput(data);

        // self.ir.0 = (irgb.red() * 0x80).into();
        // self.ir.1 = (irgb.green() * 0x80).into();
        // self.ir.2 = (irgb.blue() * 0x80).into();
    }

    /// Color conversion output
    fn orgb(&mut self, data: u32) {}

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

    let data = system.cpu.gte.data_regs[cop_r];
    debug!("mfc2: cpu_reg{cpu_r} write <- {data:x}");

    system.cpu.take_delayed_load(cpu_r, data);
}

/// Transfer from control register
fn cfc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd();

    let data = system.cpu.gte.control_regs[cop_r];
    debug!("cfc2: cpu_reg{cpu_r} write <- {data:x}");

    system.cpu.take_delayed_load(cpu_r, data);
}

/// Transfer to data register
fn mtc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd();

    let data = system.cpu.regs[cpu_r];
    debug!("mtc2: cop2r{cop_r} write <- {data:x}");

    system.cpu.gte.data_regs[cop_r] = data
}

/// Transfer to control register
fn ctc2(system: &mut System, instr: Instruction) {
    let cpu_r = instr.rt();
    let cop_r = instr.rd();

    let data = system.cpu.regs[cpu_r];
    debug!("ctc2: cop2r{} write <- {data:x}", cop_r + 32);

    system.cpu.gte.control_regs[cop_r] = data
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
    system.cpu.gte.data_regs[rt] = data;
    Ok(())
}

/// Store GTE data register
pub fn swc2(system: &mut System, instr: Instruction) -> Result<(), Exception> {
    check_valid_gte_access(system)?;

    let rs = instr.rs();
    let rt = instr.rt();
    let im = instr.imm16_se();

    let addr = system.cpu.regs[rs].wrapping_add(im);
    let data = system.cpu.gte.data_regs[rt];

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
}

bitfield::bitfield! {
#[derive(Default)]
    struct ColorConversionInput(u32);
    u16, red, _: 4, 0;
    u16, green, _: 9, 5;
    u16, blue, _: 14, 10;
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

#[derive(Default)]
struct ScreenCoordsFifo {
    fifo: ArrayVec<ScreenVector, 4>,
}

#[derive(Default)]
struct ScreenVector {
    x: FixedI16<0>,
    y: FixedI16<0>,
    z: FixedU16<0>,
}

#[derive(Default)]
struct Vector {
    x: FixedI16<0>,
    y: FixedI16<0>,
    z: FixedI16<0>,
}

type FixedU16<const FB: usize> = Fixed<u16, FB>;
type FixedI16<const FB: usize> = Fixed<i16, FB>;
type FixedU32<const FB: usize> = Fixed<u32, FB>;
type FixedI32<const FB: usize> = Fixed<i32, FB>;

#[derive(Default)]
#[repr(transparent)]
struct Fixed<T, const FB: usize>(T);

impl<T, const FB: usize> From<T> for Fixed<T, FB> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
