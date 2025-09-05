use starpsx_renderer::vec2::Vec2;

use crate::gpu::utils::{parse_clut_uv, parse_color_16, parse_page_uv, parse_uv, parse_xy};

use super::*;

impl Gpu {
    pub fn gp0_nop(&mut self) {
        // Do nothing
    }

    pub fn gp0_draw_mode(&mut self) {
        self.stat.set_page_base_x(self.gp0_params[0].page_base_x());
        self.stat.set_page_base_y(self.gp0_params[0].page_base_y());
        self.stat
            .set_semi_transparency(self.gp0_params[0].semi_transparency());
        self.stat
            .set_texture_depth(self.gp0_params[0].texture_depth());
        self.stat.set_dithering(self.gp0_params[0].dithering());
        self.stat
            .set_draw_to_display(self.gp0_params[0].draw_to_display());
        self.stat
            .set_texture_disable(self.gp0_params[0].texture_disable());
        self.texture_rect_x_flip = self.gp0_params[0].texture_rect_x_flip();
        self.texture_rect_y_flip = self.gp0_params[0].texture_rect_y_flip();
    }

    pub fn gp0_drawing_area_top_left(&mut self) {
        self.drawing_area_top = self.gp0_params[0].y_coordinates();
        self.drawing_area_left = self.gp0_params[0].x_coordinates();
    }

    pub fn gp0_drawing_area_bottom_right(&mut self) {
        self.drawing_area_bottom = self.gp0_params[0].y_coordinates();
        self.drawing_area_right = self.gp0_params[0].x_coordinates();
    }

    pub fn gp0_drawing_area_offset(&mut self) {
        self.drawing_x_offset = (self.gp0_params[0].x_offset() << 5) as i16 >> 5;
        self.drawing_y_offset = (self.gp0_params[0].y_offset() << 5) as i16 >> 5;
    }

    pub fn gp0_texture_window(&mut self) {
        self.texture_window_x_mask = self.gp0_params[0].window_mask_x();
        self.texture_window_y_mask = self.gp0_params[0].window_mask_y();

        self.texture_window_x_offset = self.gp0_params[0].window_offset_x();
        self.texture_window_y_offset = self.gp0_params[0].window_offset_y();
    }

    pub fn gp0_mask_bit_setting(&mut self) {
        self.stat
            .set_force_set_mask_bit(self.gp0_params[0].force_set_mask_bit());
        self.stat
            .set_preserve_masked_pixels(self.gp0_params[0].preserve_masked_pixels());
    }

    pub fn gp0_image_store(&mut self) {
        let resolution = self.gp0_params[2];
        let (width, height) = (resolution.image_width(), resolution.image_height());

        eprintln!("Unhandled image store of {width} x {height}");
    }

    pub fn gp0_clear_cache(&mut self) {
        // unimplemented
    }

    pub fn gp0_image_load(&mut self) {
        let (x, y) = parse_xy(self.gp0_params[1].0);
        let (width, height) = parse_xy(self.gp0_params[2].0);

        let vram_x = x as u16;
        let vram_y = y as u16;

        let width = match width as u16 {
            0 => 1024,
            x => x,
        };

        let height = match height as u16 {
            0 => 512,
            x => x,
        };

        self.gp0_state = GP0State::CopyToVram(VramCopyFields {
            vram_x,
            vram_y,
            width,
            height,
            current_row: 0,
            current_col: 0,
        });
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
        let hres = command.hres_1() | (command.hres_2() << 2);
        self.stat.set_hres(hres);
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
        self.gp0_params.clear();
        self.gp0_state = GP0State::AwaitCommand;
    }

    pub fn gp1_acknowledge_irq(&mut self) {
        self.stat.set_interrupt(false);
    }

    // DRAW COMMANDS
    pub fn gp0_quick_rect_fill(&mut self) {
        let color = parse_color_16(self.gp0_params[0].0);
        let (x, y) = parse_xy(self.gp0_params[1].0);
        let (width, height) = parse_xy(self.gp0_params[2].0);

        self.renderer.draw_rectangle_mono(
            Vec2::new(x as i32, y as i32),
            width as i32,
            height as i32,
            color,
        );
    }

    pub fn gp0_vram_to_vram_blit(&mut self) {
        let (src_x, src_y) = parse_xy(self.gp0_params[1].0);
        let (dst_x, dst_y) = parse_xy(self.gp0_params[2].0);
        let (width, height) = parse_xy(self.gp0_params[3].0);

        for y in 0..height {
            for x in 0..width {
                let src_index = 2 * ((src_y + y) * 1024 + (src_x + x)) as usize;
                let dst_index = 2 * ((dst_y + y) * 1024 + (dst_x + x)) as usize;
                self.renderer
                    .vram
                    .copy_within(src_index..(src_index + 2), dst_index);
            }
        }
    }

    pub fn gp0_quad_mono_opaque(&mut self) {
        let color = parse_color_16(self.gp0_params[0].0);
        let (x0, y0) = parse_xy(self.gp0_params[1].0);
        let (x1, y1) = parse_xy(self.gp0_params[2].0);
        let (x2, y2) = parse_xy(self.gp0_params[3].0);
        let (x3, y3) = parse_xy(self.gp0_params[4].0);

        let quad = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
            Vec2::new(x3 as i32, y3 as i32),
        ];

        self.renderer.draw_quad_mono_opaque(quad, color);
    }

    pub fn gp0_quad_shaded_opaque(&mut self) {
        let (x0, y0) = parse_xy(self.gp0_params[1].0);
        let (x1, y1) = parse_xy(self.gp0_params[3].0);
        let (x2, y2) = parse_xy(self.gp0_params[5].0);
        let (x3, y3) = parse_xy(self.gp0_params[7].0);

        let quad = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
            Vec2::new(x3 as i32, y3 as i32),
        ];

        let colors = [
            parse_color_16(self.gp0_params[0].0),
            parse_color_16(self.gp0_params[2].0),
            parse_color_16(self.gp0_params[4].0),
            parse_color_16(self.gp0_params[6].0),
        ];

        self.renderer.draw_quad_shaded_opaque(quad, colors);
    }

    pub fn gp0_triangle_shaded_opaque(&mut self) {
        let (x0, y0) = parse_xy(self.gp0_params[1].0);
        let (x1, y1) = parse_xy(self.gp0_params[3].0);
        let (x2, y2) = parse_xy(self.gp0_params[5].0);

        let triangle = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
        ];

        let colors = [
            parse_color_16(self.gp0_params[0].0),
            parse_color_16(self.gp0_params[2].0),
            parse_color_16(self.gp0_params[4].0),
        ];

        self.renderer.draw_triangle_shaded_opaque(triangle, colors);
    }

    pub fn gp0_quad_texture_blend_opaque(&mut self) {
        let (x0, y0) = parse_xy(self.gp0_params[1].0);
        let (x1, y1) = parse_xy(self.gp0_params[3].0);
        let (x2, y2) = parse_xy(self.gp0_params[5].0);
        let (x3, y3) = parse_xy(self.gp0_params[7].0);

        let quad = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
            Vec2::new(x3 as i32, y3 as i32),
        ];

        let (clut, u0, v0) = parse_clut_uv(self.gp0_params[2].0);
        let (texture, u1, v1) = parse_page_uv(self.gp0_params[4].0, clut);
        let (u2, v2) = parse_uv(self.gp0_params[6].0);
        let (u3, v3) = parse_uv(self.gp0_params[8].0);

        let uvs = [
            Vec2::new(u0 as i32, v0 as i32),
            Vec2::new(u1 as i32, v1 as i32),
            Vec2::new(u2 as i32, v2 as i32),
            Vec2::new(u3 as i32, v3 as i32),
        ];

        self.renderer
            .draw_quad_texture_blend_opaque(quad, uvs, texture);
    }

    pub fn gp0_draw_1x1_rectangle(&mut self) {
        let color = parse_color_16(self.gp0_params[0].0);
        let (x, y) = parse_xy(self.gp0_params[1].0);
        self.renderer
            .draw_rectangle_mono(Vec2::new(x as i32, y as i32), 1, 1, color);
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
