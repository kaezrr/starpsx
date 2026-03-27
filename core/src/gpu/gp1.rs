use starpsx_renderer::vec2::Vec2;

use super::Command;
use super::GP0State;
use super::Gpu;
use super::VerticalRes;

impl Gpu {
    pub fn gp1_reset(&mut self) {
        self.status.0 = 0x1480_2000;
        self.status.set_hres(1);
        self.renderer.ctx.reset();

        // NOTE: Clear command cache and invalidate GPU cache here if I ever implement it
        self.gp1_reset_command_buffer();
    }

    pub fn gp1_display_mode(&mut self, command: Command) {
        let hres = command.hres_1() | (command.hres_2() << 2);

        self.status.set_hres(hres);
        self.status.set_display_depth(command.display_depth());

        self.status.set_interlaced_v(command.interlaced());
        self.status.set_interlaced(command.interlaced());

        self.renderer.ctx.display_depth = command.display_depth();
        self.renderer.ctx.interlaced = command.interlaced();

        // Free to set whatever vertical resolution if interlaced
        if command.interlaced() {
            self.status.set_vres(command.vres());
        } else {
            self.status.set_vres(VerticalRes::Y240);
        }

        let width = self.status.hres().as_value();
        let height = self.status.vres().as_value();

        self.renderer.change_resolution(width, height);
        self.status.set_vmode(command.vmode());

        if command.flip_screen() {
            unimplemented!("Flip screen bit not supported!");
        }
    }

    pub fn gp1_dma_direction(&mut self, command: Command) {
        self.status.set_dma_direction(command.dma_direction());
    }

    pub fn gp1_display_vram_start(&mut self, command: Command) {
        self.renderer.ctx.display_vram_start = {
            let x = command.display_vram_x() & !1; // LSB is ignored? not sure
            let y = command.display_vram_y();
            Vec2::new(i32::from(x), i32::from(y))
        };
    }

    pub fn gp1_display_horizontal_range(&mut self, command: Command) {
        let x1 = command.horizontal_x1();
        let x2 = command.horizontal_x2();

        let dotclock = self.get_dot_clock_divider();
        self.renderer.ctx.display_hor_range = (((x2 - x1) / dotclock) + 2) & !3;
    }

    pub fn gp1_display_vertical_range(&mut self, command: Command) {
        let y1 = command.vertical_y1();
        let y2 = command.vertical_y2();

        self.renderer.ctx.display_ver_range = y2 - y1;
    }

    pub fn gp1_display_enable(&mut self, command: Command) {
        self.status.set_display_disabled(command.display_off());
        self.renderer.ctx.display_disabled = command.display_off();
    }

    pub fn gp1_reset_command_buffer(&mut self) {
        self.state = GP0State::AwaitCommand;
    }

    pub fn gp1_acknowledge_irq(&mut self) {
        self.status.set_interrupt(false);
    }

    // GPU version v0
    pub fn gp1_read_internal_reg(&mut self, command: Command) {
        self.read = match command.register_index() & 0x7 {
            0x00 | 0x01 | 0x07 | 0x06 => self.read,
            0x02 => todo!("Read texture window setting"),
            0x03 => self.draw_area_top_left(),
            0x04 => self.draw_area_bottom_right(),
            0x05 => self.draw_offset(),
            _ => unreachable!("0x07 mod cannot reach here"),
        };
    }
}
