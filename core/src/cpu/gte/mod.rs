mod commands;

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
