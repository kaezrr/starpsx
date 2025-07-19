use crate::gpu::{Command, Gpu};

impl Gpu {
    pub fn gpu0_draw_mode(&mut self, command: Command) {
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

    pub fn gpu1_reset(&mut self) {
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

        // Clear command cache and invalidate GPU cache if i ever implement it
    }
}
