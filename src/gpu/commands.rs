use crate::gpu::{Command, GP0State, Gpu};

impl Gpu {
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
    }

    pub fn gp0_nop(&mut self) {
        // Do nothing
    }

    pub fn gp0_draw_mode(&mut self) {
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
    }

    pub fn gp0_drawing_area_top_left(&mut self) {
        self.drawing_area_top = self.commands[0].y_coordinates();
        self.drawing_area_left = self.commands[0].x_coordinates();
    }

    pub fn gp0_drawing_area_bottom_right(&mut self) {
        self.drawing_area_bottom = self.commands[0].y_coordinates();
        self.drawing_area_right = self.commands[0].x_coordinates();
    }

    pub fn gp0_drawing_area_offset(&mut self) {
        self.drawing_x_offset = (self.commands[0].x_offset() << 5) as i16 >> 5;
        self.drawing_y_offset = (self.commands[0].y_offset() << 5) as i16 >> 5;
    }

    pub fn gp0_texture_window(&mut self) {
        self.texture_window_x_mask = self.commands[0].window_mask_x();
        self.texture_window_y_mask = self.commands[0].window_mask_y();

        self.texture_window_x_offset = self.commands[0].window_offset_x();
        self.texture_window_y_offset = self.commands[0].window_offset_y();
    }

    pub fn gp0_mask_bit_setting(&mut self) {
        self.stat
            .set_force_set_mask_bit(self.commands[0].force_set_mask_bit());
        self.stat
            .set_preserve_masked_pixels(self.commands[0].preserve_masked_pixels());
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

    pub fn gp0_quad_mono_opaque(&mut self) {
        eprintln!("draw quad");
    }

    pub fn gp0_clear_cache(&mut self) {
        // unimplemented
    }

    pub fn gp0_image_load(&mut self) {
        let image_size = self.commands[2].image_width() * self.commands[2].image_height();
        let image_size = (image_size + 1) & !1;

        self.args_len = (image_size / 2) as usize;
        self.gp0_state = GP0State::LoadImage;
    }
}
