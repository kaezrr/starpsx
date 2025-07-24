use crate::gpu::{Command, Gpu};

impl Gpu {
    pub fn gp0_draw_mode(&mut self, command: Command) {
        self.stat.set_page_base_x(command.page_base_x());
        self.stat.set_page_base_y(command.page_base_y());
        self.stat.set_semi_transparency(command.semi_transparency());
        self.stat.set_texture_depth(command.texture_depth());
        self.stat.set_dithering(command.dithering());
        self.stat.set_draw_to_display(command.draw_to_display());
        self.stat.set_texture_disable(command.texture_disable());
        self.texture_rect_x_flip = command.texture_rect_x_flip();
        self.texture_rect_y_flip = command.texture_rect_y_flip();
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

    pub fn gp0_drawing_area_top_left(&mut self, command: Command) {
        self.drawing_area_top = command.y_coordinates();
        self.drawing_area_left = command.x_coordinates();
    }

    pub fn gp0_drawing_area_bottom_right(&mut self, command: Command) {
        self.drawing_area_bottom = command.y_coordinates();
        self.drawing_area_right = command.x_coordinates();
    }

    pub fn gp0_drawing_area_offset(&mut self, command: Command) {
        self.drawing_x_offset = (command.x_offset() << 5) as i16 >> 5;
        self.drawing_y_offset = (command.y_offset() << 5) as i16 >> 5;
    }

    pub fn gp0_texture_window(&mut self, command: Command) {
        self.texture_window_x_mask = command.window_mask_x();
        self.texture_window_y_mask = command.window_mask_y();

        self.texture_window_x_offset = command.window_offset_x();
        self.texture_window_y_offset = command.window_offset_y();
    }

    pub fn gp0_mask_bit_setting(&mut self, command: Command) {
        self.stat
            .set_force_set_mask_bit(command.force_set_mask_bit());
        self.stat
            .set_preserve_masked_pixels(command.preserve_masked_pixels());
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
}
