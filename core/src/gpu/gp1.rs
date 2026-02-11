use super::*;
use starpsx_renderer::vec2::Vec2;

impl Gpu {
    pub fn gp1_reset(&mut self) {
        self.gpu_stat.0 = 0x14802000;
        self.gpu_stat.set_hres(1);
        self.renderer.ctx.reset();

        // NOTE: Clear command cache and invalidate GPU cache here if I ever implement it
        self.gp1_reset_command_buffer();
    }

    pub fn gp1_display_mode(&mut self, command: Command) {
        let hres = command.hres_1() | (command.hres_2() << 2);

        self.gpu_stat.set_hres(hres);

        self.gpu_stat.set_vmode(command.vmode());
        self.gpu_stat.set_display_depth(command.display_depth());

        self.gpu_stat.set_interlaced_v(command.interlaced());
        self.gpu_stat.set_interlaced(command.interlaced());

        self.renderer.ctx.display_depth = command.display_depth();
        self.renderer.ctx.interlaced = command.interlaced();

        // Free to set whatever vertical resolution if interlaced
        if command.interlaced() {
            self.gpu_stat.set_vres(command.vres());
        } else {
            self.gpu_stat.set_vres(VerticalRes::Y240);
        }

        let width = self.gpu_stat.hres().as_value();
        let height = self.gpu_stat.vres().as_value();

        self.renderer.change_resolution(width, height);

        if command.flip_screen() {
            unimplemented!("Flip screen bit not supported!");
        }
    }

    pub fn gp1_dma_direction(&mut self, command: Command) {
        self.gpu_stat.set_dma_direction(command.dma_direction());
    }

    pub fn gp1_display_vram_start(&mut self, command: Command) {
        self.renderer.ctx.display_vram_start = {
            let x = command.display_vram_x() & !1; // LSB is ignored? not sure
            let y = command.display_vram_y();
            Vec2::new(x as i32, y as i32)
        };
    }

    pub fn gp1_display_horizontal_range(&mut self, command: Command) {
        let x1 = command.horizontal_x1().max(0x260); // 608
        let x2 = command.horizontal_x2().min(0x260 + 320 * 8); // 3168

        let dotclock = self.get_dot_clock_divider();

        self.renderer.ctx.display_x1 = (x1 - 488) / dotclock;
        self.renderer.ctx.display_x2 = (x2 - 488) / dotclock;
    }

    pub fn gp1_display_vertical_range(&mut self, command: Command) {
        let y1 = command.vertical_y1().max(0x88 - 240 / 2); // 16
        let y2 = command.vertical_y2().min(0x88 + 240 / 2); // 256

        let mul = if self.renderer.ctx.interlaced { 2 } else { 1 };

        self.renderer.ctx.display_y1 = (y1 - 16) * mul;
        self.renderer.ctx.display_y2 = (y2 - 16) * mul;
    }

    pub fn gp1_display_enable(&mut self, command: Command) {
        self.gpu_stat.set_display_disabled(command.display_off());
        self.renderer.ctx.display_disabled = command.display_off();
    }

    pub fn gp1_reset_command_buffer(&mut self) {
        self.gp0_state = GP0State::AwaitCommand;
    }

    pub fn gp1_acknowledge_irq(&mut self) {
        self.gpu_stat.set_interrupt(false);
    }

    // GPU version v0
    pub fn gp1_read_internal_reg(&mut self, command: Command) {
        self.gpu_read = match command.register_index() & 0x7 {
            0x00 | 0x01 | 0x07 | 0x06 => self.gpu_read,
            0x02 => todo!("Read texture window setting"),
            0x03 => self.draw_area_top_left(),
            0x04 => self.draw_area_bottom_right(),
            0x05 => self.draw_offset(),
            _ => unreachable!("0x07 mod cannot reach here"),
        };
    }
}
