use cpu::Cpu;
use memory::Bus;
use std::error::Error;

mod cpu;
mod dma;
pub mod gpu;
mod memory;

pub const TARGET_FPS: u64 = 60;
const MCYCLES_PER_SECOND: u32 = 564480;

pub struct Config {
    pub bios_path: String,
    pub exe_path: Option<String>,
}

impl Config {
    pub fn build() -> Result<Config, Box<dyn Error>> {
        let args: Vec<String> = std::env::args().collect();

        let bios_path = match args.get(1) {
            Some(x) => x.clone(),
            None => return Err("missing bios path".into()),
        };
        let exe_path = args.get(2).cloned();

        Ok(Config {
            bios_path,
            exe_path,
        })
    }
}

pub struct StarPSX {
    cpu: Cpu,
    bus: Bus,
    tty: String,
}

impl StarPSX {
    pub fn build(config: Config) -> Result<Self, Box<dyn Error>> {
        let bus = Bus::build(&config)?;
        let cpu = Cpu::default();
        let mut psx = StarPSX {
            cpu,
            bus,
            tty: String::new(),
        };
        if let Some(exe_path) = config.exe_path {
            psx.sideload_exe(&exe_path)?;
        }
        Ok(psx)
    }

    pub fn frame_buffer(&self) -> &[u32] {
        let (width, height) = self.bus.gpu.get_resolution();
        &self.bus.gpu.renderer.frame_buffer()[..(width * height)]
    }

    pub fn frame_buffer_vram(&self) -> &[u32] {
        self.bus.gpu.renderer.frame_buffer()
    }

    pub fn get_resolution(&self) -> (u32, u32) {
        let (width, height) = self.bus.gpu.get_resolution();
        (width as u32, height as u32)
    }

    pub fn step_frame(&mut self) {
        for _ in (0..MCYCLES_PER_SECOND).step_by(2) {
            self.cpu.run_instruction(&mut self.bus);
            self.check_for_tty_output();
        }
        self.bus.irqctl.stat().set_vblank(true);
        self.bus.gpu.renderer.copy_display_to_fb();
    }

    pub fn sideload_exe(&mut self, filepath: &String) -> Result<(), Box<dyn Error>> {
        let exe = std::fs::read(filepath)?;
        while self.cpu.pc != 0x80030000 {
            self.cpu.run_instruction(&mut self.bus);
            self.check_for_tty_output();
        }

        // Parse EXE header
        let init_pc = u32::from_le_bytes(exe[0x10..0x14].try_into().unwrap());
        let init_r28 = u32::from_le_bytes(exe[0x14..0x18].try_into().unwrap());
        let exe_addr = u32::from_le_bytes(exe[0x18..0x1C].try_into().unwrap()) & 0x1FFFFF;
        let exe_size = u32::from_le_bytes(exe[0x1C..0x20].try_into().unwrap());
        let init_sp = u32::from_le_bytes(exe[0x30..0x34].try_into().unwrap());

        // Copy EXE data to RAM
        self.bus.ram.bytes[exe_addr as usize..(exe_addr + exe_size) as usize]
            .copy_from_slice(&exe[2048..2048 + exe_size as usize]);

        self.cpu.regs[28] = init_r28;
        if init_sp != 0 {
            self.cpu.regs[29] = init_sp;
            self.cpu.regs[30] = init_sp;
        }

        self.cpu.pc = init_pc;

        Ok(())
    }

    fn check_for_tty_output(&mut self) {
        let pc = self.cpu.pc & 0x1FFFFFFF;
        if (pc == 0xA0 && self.cpu.regs[9] == 0x3C) || (pc == 0xB0 && self.cpu.regs[9] == 0x3D) {
            let ch = self.cpu.regs[4] as u8 as char;
            self.tty.push(ch);
            if ch == '\n' {
                print!("{}", self.tty);
                self.tty = String::new();
            }
        }
    }
}
