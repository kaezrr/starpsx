use super::*;

impl Gpu {
    pub fn gp0_nop(&mut self) -> GP0State {
        GP0State::AwaitCommand
    }

    pub fn gp0_draw_mode(&mut self) -> GP0State {
        self.stat.set_page_base_x(self.commands[0].page_base_x());
        self.stat.set_page_base_y(self.commands[0].page_base_y());
        self.stat
            .set_semi_transparency(self.commands[0].semi_transparency());
        self.stat
            .set_texture_depth(self.commands[0].texture_depth());
        self.stat.set_dithering(self.commands[0].dithering());
        self.stat
            .set_draw_to_display(self.commands[0].draw_to_display());
        self.stat
            .set_texture_disable(self.commands[0].texture_disable());
        self.texture_rect_x_flip = self.commands[0].texture_rect_x_flip();
        self.texture_rect_y_flip = self.commands[0].texture_rect_y_flip();
        GP0State::AwaitCommand
    }

    pub fn gp0_drawing_area_top_left(&mut self) -> GP0State {
        self.drawing_area_top = self.commands[0].y_coordinates();
        self.drawing_area_left = self.commands[0].x_coordinates();
        GP0State::AwaitCommand
    }

    pub fn gp0_drawing_area_bottom_right(&mut self) -> GP0State {
        self.drawing_area_bottom = self.commands[0].y_coordinates();
        self.drawing_area_right = self.commands[0].x_coordinates();
        GP0State::AwaitCommand
    }

    pub fn gp0_drawing_area_offset(&mut self) -> GP0State {
        self.drawing_x_offset = (self.commands[0].x_offset() << 5) as i16 >> 5;
        self.drawing_y_offset = (self.commands[0].y_offset() << 5) as i16 >> 5;
        GP0State::AwaitCommand
    }

    pub fn gp0_texture_window(&mut self) -> GP0State {
        self.texture_window_x_mask = self.commands[0].window_mask_x();
        self.texture_window_y_mask = self.commands[0].window_mask_y();

        self.texture_window_x_offset = self.commands[0].window_offset_x();
        self.texture_window_y_offset = self.commands[0].window_offset_y();
        GP0State::AwaitCommand
    }

    pub fn gp0_mask_bit_setting(&mut self) -> GP0State {
        self.stat
            .set_force_set_mask_bit(self.commands[0].force_set_mask_bit());
        self.stat
            .set_preserve_masked_pixels(self.commands[0].preserve_masked_pixels());
        GP0State::AwaitCommand
    }

    pub fn gp0_image_store(&mut self) -> GP0State {
        let resolution = self.commands[2];
        let (width, height) = (resolution.image_width(), resolution.image_height());
        // let image_size = resolution.image_width() * resolution.image_height();

        eprintln!("Unhandled image store of {width} x {height}");
        GP0State::AwaitCommand
    }

    pub fn gp0_clear_cache(&mut self) -> GP0State {
        // unimplemented
        GP0State::AwaitCommand
    }

    pub fn gp0_image_load(&mut self) -> GP0State {
        let vram_x = (self.commands[0].0 & 0x3FF) as u16;
        let vram_y = ((self.commands[0].0 >> 16) & 0x1FF) as u16;

        let width = match (self.commands[2].0 & 0x3FF) as u16 {
            0 => 1024,
            x => x,
        };

        let height = match ((self.commands[2].0 >> 16) & 0x3FF) as u16 {
            0 => 512,
            x => x,
        };

        GP0State::CopyToVram(VramCopyFields {
            vram_x,
            vram_y,
            width,
            height,
            current_row: 0,
            current_col: 0,
        })
    }

    pub fn gp1_reset(&mut self) {
        self.texture_rect_x_flip = false;
        self.texture_rect_y_flip = false;

        self.texture_window_x_mask = 0;
        self.texture_window_y_mask = 0;

        self.texture_window_x_offset = 0;
        self.texture_window_y_offset = 0;

        self.drawing_area_left = 0;
        self.drawing_area_top = 0;
        self.drawing_area_right = 0;
        self.drawing_area_bottom = 0;

        self.drawing_x_offset = 0;
        self.drawing_y_offset = 0;

        self.display_vram_x_start = 0;
        self.display_vram_y_start = 0;

        self.display_hori_start = 0x200;
        self.display_hori_end = 0xc00;

        self.display_line_start = 0x10;
        self.display_line_end = 0x100;

        self.stat.0 = 0;
        // NOTE: Clear command cache and invalidate GPU cache here if I ever implement it

        self.gp1_reset_command_buffer();
    }

    pub fn gp1_display_mode(&mut self, command: Command) {
        self.stat
            .set_hres((command.hres_1() << 1) | command.hres_2());
        self.stat.set_vres(command.vres());
        self.stat.set_vmode(command.vmode());
        self.stat.set_display_depth(command.display_depth());
        self.stat.set_interlaced(command.interlaced());

        if command.flip_screen() {
            panic!("Flip screen bit not supported!");
        }
    }

    pub fn gp1_dma_direction(&mut self, command: Command) {
        self.stat.set_dma_direction(command.dma_direction());
    }

    pub fn gp1_display_vram_start(&mut self, command: Command) {
        self.display_vram_x_start = command.display_vram_x();
        self.display_vram_y_start = command.display_vram_y();
    }

    pub fn gp1_display_horizontal_range(&mut self, command: Command) {
        self.display_hori_start = command.horizontal_x1();
        self.display_hori_end = command.horizontal_x2();
    }

    pub fn gp1_display_vertical_range(&mut self, command: Command) {
        self.display_line_start = command.vertical_y1();
        self.display_line_end = command.vertical_y2();
    }

    pub fn gp1_display_enable(&mut self, command: Command) {
        self.stat.set_display_disabled(command.display_off());
    }

    pub fn gp1_reset_command_buffer(&mut self) {
        self.commands.clear();
        self.gp0_state = GP0State::AwaitCommand;
    }

    // DRAW COMMANDS
    pub fn gp1_acknowledge_irq(&mut self) {
        self.stat.set_interrupt(false);
    }

    pub fn gp0_quad_mono_opaque(&mut self) -> GP0State {
        eprintln!("draw quad");
        GP0State::AwaitCommand
    }

    pub fn gp0_quad_shaded_opaque(&mut self) -> GP0State {
        eprintln!("draw shaded quad");
        GP0State::AwaitCommand
    }

    pub fn gp0_triangle_shaded_opaque(&mut self) -> GP0State {
        eprintln!("draw shaded triangle");
        GP0State::AwaitCommand
    }

    pub fn gp0_quad_texture_blend_opaque(&mut self) -> GP0State {
        eprintln!("draw texture blended quad");
        GP0State::AwaitCommand
    }
}
