use super::utils::{parse_clut_uv, parse_page_uv, parse_uv, parse_xy};
use super::*;
use starpsx_renderer::{utils::Color, vec2::Vec2};

impl Gpu {
    pub fn gp0_nop(&mut self, _params: ArrayVec<Command, 16>) -> GP0State {
        // Do nothing
        GP0State::AwaitCommand
    }

    pub fn gp0_clear_cache(&mut self, _params: ArrayVec<Command, 16>) -> GP0State {
        // unimplemented
        GP0State::AwaitCommand
    }

    pub fn gp0_draw_mode(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        self.stat.set_page_base_x(params[0].page_base_x());
        self.stat.set_page_base_y(params[0].page_base_y());
        self.stat
            .set_semi_transparency(params[0].semi_transparency());
        self.stat.set_texture_depth(params[0].texture_depth());
        self.stat.set_dithering(params[0].dithering());
        self.stat.set_draw_to_display(params[0].draw_to_display());
        self.stat.set_texture_disable(params[0].texture_disable());

        self.renderer.ctx.texture_rect_x_flip = params[0].texture_rect_x_flip();
        self.renderer.ctx.texture_rect_y_flip = params[0].texture_rect_y_flip();
        self.renderer.ctx.dithering = self.stat.dithering();
        self.renderer.ctx.transparency_weights = match self.stat.semi_transparency() {
            0 => (0.5, 0.5),
            1 => (1.0, 1.0),
            2 => (1.0, -1.0),
            3 => (1.0, 0.25),
            _ => unreachable!("2 bit value cant reach here"),
        };
        GP0State::AwaitCommand
    }

    pub fn gp0_drawing_area_top_left(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        self.renderer.ctx.drawing_area_top_left = {
            let x = params[0].x_coordinates();
            let y = params[0].y_coordinates();
            Vec2::new(x as i32, y as i32)
        };
        GP0State::AwaitCommand
    }

    pub fn gp0_drawing_area_bottom_right(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        self.renderer.ctx.drawing_area_bottom_right = {
            let x = params[0].x_coordinates();
            let y = params[0].y_coordinates();
            Vec2::new(x as i32, y as i32)
        };
        GP0State::AwaitCommand
    }

    pub fn gp0_drawing_area_offset(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        self.renderer.ctx.drawing_area_offset = {
            let x = (params[0].x_offset() << 5) as i16 >> 5;
            let y = (params[0].y_offset() << 5) as i16 >> 5;
            Vec2::new(x as i32, y as i32)
        };
        GP0State::AwaitCommand
    }

    pub fn gp0_texture_window(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        self.renderer.ctx.texture_window_mask = {
            let x = params[0].window_mask_x();
            let y = params[0].window_mask_y();
            Vec2::new(x as i32, y as i32)
        };

        self.renderer.ctx.texture_window_offset = {
            let x = params[0].window_offset_x();
            let y = params[0].window_offset_y();
            Vec2::new(x as i32, y as i32)
        };
        GP0State::AwaitCommand
    }

    pub fn gp0_mask_bit_setting(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        self.stat
            .set_force_set_mask_bit(params[0].force_set_mask_bit());
        self.stat
            .set_preserve_masked_pixels(params[0].preserve_masked_pixels());
        GP0State::AwaitCommand
    }

    pub fn gp0_image_store(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x, y) = parse_xy(params[1].0);
        let (width, height) = parse_xy(params[2].0);

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

        GP0State::CopyFromVram(VramCopyFields {
            vram_x,
            vram_y,
            width,
            height,
            current_row: 0,
            current_col: 0,
        })
    }

    pub fn gp0_image_load(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x, y) = parse_xy(params[1].0);
        let (width, height) = parse_xy(params[2].0);

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

        GP0State::CopyToVram(VramCopyFields {
            vram_x,
            vram_y,
            width,
            height,
            current_row: 0,
            current_col: 0,
        })
    }
    // DRAW COMMANDS
    pub fn gp0_quick_rect_fill(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let (x, y) = parse_xy(params[1].0);
        let (width, height) = parse_xy(params[2].0);

        self.renderer.draw_rectangle_mono(
            Vec2::new(x as i32, y as i32),
            width as i32,
            height as i32,
            color,
        );
        GP0State::AwaitCommand
    }

    pub fn gp0_vram_to_vram_blit(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (src_x, src_y) = parse_xy(params[1].0);
        let (dst_x, dst_y) = parse_xy(params[2].0);
        let (width, height) = parse_xy(params[3].0);

        self.renderer.vram_self_copy(
            src_x as usize,
            src_y as usize,
            dst_x as usize,
            dst_y as usize,
            width as usize,
            height as usize,
        );
        GP0State::AwaitCommand
    }

    pub fn gp0_triangle_mono_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[2].0);
        let (x2, y2) = parse_xy(params[3].0);

        let triangle = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
        ];

        self.renderer.draw_triangle_mono_opaque(triangle, color);
        GP0State::AwaitCommand
    }

    pub fn gp0_triangle_mono_trans(&mut self, _params: ArrayVec<Command, 16>) -> GP0State {
        println!("gp0_triangle_mono_trans");
        GP0State::AwaitCommand
    }

    pub fn gp0_quad_mono_trans(&mut self, _params: ArrayVec<Command, 16>) -> GP0State {
        println!("gp0_quad_mono_trans");
        GP0State::AwaitCommand
    }

    pub fn gp0_quad_mono_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[2].0);
        let (x2, y2) = parse_xy(params[3].0);
        let (x3, y3) = parse_xy(params[4].0);

        let quad = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
            Vec2::new(x3 as i32, y3 as i32),
        ];

        self.renderer.draw_quad_mono_opaque(quad, color);
        GP0State::AwaitCommand
    }

    pub fn gp0_quad_shaded_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[3].0);
        let (x2, y2) = parse_xy(params[5].0);
        let (x3, y3) = parse_xy(params[7].0);

        let quad = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
            Vec2::new(x3 as i32, y3 as i32),
        ];

        let colors = [
            Color::new_8bit(params[0].0).to_5bit(),
            Color::new_8bit(params[2].0).to_5bit(),
            Color::new_8bit(params[4].0).to_5bit(),
            Color::new_8bit(params[6].0).to_5bit(),
        ];

        self.renderer.draw_quad_shaded_opaque(quad, colors);
        GP0State::AwaitCommand
    }

    pub fn gp0_triangle_shaded_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[3].0);
        let (x2, y2) = parse_xy(params[5].0);

        let triangle = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
        ];

        let colors = [
            Color::new_8bit(params[0].0).to_5bit(),
            Color::new_8bit(params[2].0).to_5bit(),
            Color::new_8bit(params[4].0).to_5bit(),
        ];

        self.renderer.draw_triangle_shaded_opaque(triangle, colors);
        GP0State::AwaitCommand
    }

    pub fn gp0_triangle_shaded_trans(&mut self, _params: ArrayVec<Command, 16>) -> GP0State {
        println!("gp0_triangle_shaded_trans");
        GP0State::AwaitCommand
    }

    pub fn gp0_quad_shaded_trans(&mut self, _params: ArrayVec<Command, 16>) -> GP0State {
        println!("gp0_quad_shaded_trans");
        GP0State::AwaitCommand
    }

    pub fn gp0_quad_texture_blend_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[3].0);
        let (x2, y2) = parse_xy(params[5].0);
        let (x3, y3) = parse_xy(params[7].0);

        let quad = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
            Vec2::new(x2 as i32, y2 as i32),
            Vec2::new(x3 as i32, y3 as i32),
        ];

        let (clut, u0, v0) = parse_clut_uv(params[2].0);
        let (texture, u1, v1) = parse_page_uv(params[4].0, clut);
        let (u2, v2) = parse_uv(params[6].0);
        let (u3, v3) = parse_uv(params[8].0);

        let uvs = [
            Vec2::new(u0 as i32, v0 as i32),
            Vec2::new(u1 as i32, v1 as i32),
            Vec2::new(u2 as i32, v2 as i32),
            Vec2::new(u3 as i32, v3 as i32),
        ];

        self.renderer
            .draw_quad_texture_blend_opaque(quad, uvs, texture);
        GP0State::AwaitCommand
    }

    pub fn gp0_draw_1x1_rectangle(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let (x, y) = parse_xy(params[1].0);
        self.renderer
            .draw_rectangle_mono(Vec2::new(x as i32, y as i32), 1, 1, color);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_mono_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[2].0);
        let color = Color::new_8bit(params[0].0).to_5bit();

        let line = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
        ];

        self.renderer.draw_line_mono(line, color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_mono_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[2].0);
        let color = Color::new_8bit(params[0].0).to_5bit();

        let line = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
        ];

        self.renderer.draw_line_mono(line, color, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_shaded_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[3].0);

        let line = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
        ];

        let colors = [
            Color::new_8bit(params[0].0).to_5bit(),
            Color::new_8bit(params[2].0).to_5bit(),
        ];

        self.renderer.draw_line_shaded(line, colors, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_shaded_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let (x0, y0) = parse_xy(params[1].0);
        let (x1, y1) = parse_xy(params[3].0);

        let line = [
            Vec2::new(x0 as i32, y0 as i32),
            Vec2::new(x1 as i32, y1 as i32),
        ];

        let colors = [
            Color::new_8bit(params[0].0).to_5bit(),
            Color::new_8bit(params[2].0).to_5bit(),
        ];

        self.renderer.draw_line_shaded(line, colors, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_mono_opaque(&mut self, vertices: Vec<u32>, colors: Vec<u32>) -> GP0State {
        let vertices = vertices
            .into_iter()
            .map(|word| {
                let (x, y) = parse_xy(word);
                Vec2::new(x as i32, y as i32)
            })
            .collect();

        let color = Color::new_8bit(colors[0]).to_5bit();
        self.renderer.draw_line_poly_mono(vertices, color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_mono_trans(&mut self, vertices: Vec<u32>, colors: Vec<u32>) -> GP0State {
        let vertices = vertices
            .into_iter()
            .map(|word| {
                let (x, y) = parse_xy(word);
                Vec2::new(x as i32, y as i32)
            })
            .collect();

        let color = Color::new_8bit(colors[0]).to_5bit();
        self.renderer.draw_line_poly_mono(vertices, color, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_shaded_opaque(
        &mut self,
        vertices: Vec<u32>,
        colors: Vec<u32>,
    ) -> GP0State {
        let vertices = vertices
            .into_iter()
            .map(|word| {
                let (x, y) = parse_xy(word);
                Vec2::new(x as i32, y as i32)
            })
            .collect();

        let colors = colors
            .into_iter()
            .map(|word| Color::new_8bit(word).to_5bit())
            .collect();

        self.renderer.draw_line_poly_shaded(vertices, colors, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_shaded_trans(&mut self, vertices: Vec<u32>, colors: Vec<u32>) -> GP0State {
        let vertices = vertices
            .into_iter()
            .map(|word| {
                let (x, y) = parse_xy(word);
                Vec2::new(x as i32, y as i32)
            })
            .collect();

        let colors = colors
            .into_iter()
            .map(|word| Color::new_8bit(word).to_5bit())
            .collect();

        self.renderer.draw_line_poly_shaded(vertices, colors, true);
        GP0State::AwaitCommand
    }
}
