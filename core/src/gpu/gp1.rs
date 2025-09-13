use super::*;
use starpsx_renderer::vec2::Vec2;

impl Gpu {
    pub fn gp1_reset(&mut self) {
        self.gpu_stat.0 = 0;
        self.renderer.ctx.reset();

        // NOTE: Clear command cache and invalidate GPU cache here if I ever implement it
        self.gp1_reset_command_buffer();
    }

    pub fn gp1_display_mode(&mut self, command: Command) {
        let hres = command.hres_1() | (command.hres_2() << 2);
        self.gpu_stat.set_hres(hres);
        self.gpu_stat.set_vres(command.vres());
        self.gpu_stat.set_vmode(command.vmode());
        self.gpu_stat.set_display_depth(command.display_depth());
        self.gpu_stat.set_interlaced(command.interlaced());

        let (x, y) = self.get_resolution();
        self.renderer.ctx.resolution = Vec2::new(x as i32, y as i32);
        self.renderer.ctx.display_depth = command.display_depth();

        if command.flip_screen() {
            panic!("Flip screen bit not supported!");
        }
    }

    pub fn gp1_dma_direction(&mut self, command: Command) {
        self.gpu_stat.set_dma_direction(command.dma_direction());
    }

    pub fn gp1_display_vram_start(&mut self, command: Command) {
        self.renderer.ctx.display_vram_start = {
            let x = command.display_vram_x();
            let y = command.display_vram_y();
            Vec2::new(x as i32, y as i32)
        };
    }

    pub fn gp1_display_horizontal_range(&mut self, command: Command) {
        self.renderer.ctx.display_hori_range = {
            let x = command.horizontal_x1();
            let y = command.horizontal_x2();
            Vec2::new(x as i32, y as i32)
        };
    }

    pub fn gp1_display_vertical_range(&mut self, command: Command) {
        self.renderer.ctx.display_line_range = {
            let x = command.vertical_y1();
            let y = command.vertical_y2();
            Vec2::new(x as i32, y as i32)
        };
    }

    pub fn gp1_display_enable(&mut self, command: Command) {
        self.gpu_stat.set_display_disabled(command.display_off());
    }

    pub fn gp1_reset_command_buffer(&mut self) {
        self.gp0_state = GP0State::AwaitCommand;
    }

    pub fn gp1_acknowledge_irq(&mut self) {
        self.gpu_stat.set_interrupt(false);
    }

    pub fn gp1_read_internal_reg(&mut self, command: Command) {
        self.gpu_read = match command.register_index() & 0x0F {
            0x00 | 0x01 | 0x06 | 0x09..=0x0F => self.gpu_read,
            0x02 => todo!("Read texture window setting"),
            0x03 => todo!("Read draw area top left"),
            0x04 => todo!("Read draw area bottom right"),
            0x05 => todo!("Read draw offset"),
            0x07 => 0x000000002, // GPU Version
            0x08 => 0x000000000, // Unknown
            _ => unreachable!("0x0F mod cannot reach here"),
        };
    }
}
