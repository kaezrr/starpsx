use super::utils::{parse_clut_uv, parse_page_uv, parse_uv, parse_xy};
use super::*;
use starpsx_renderer::{ColorOptions, DrawOptions};
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
        let Vec2 { x, y } = parse_xy(params[1].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[2].0);
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
        let Vec2 { x, y } = parse_xy(params[1].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[2].0);

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
        let v = parse_xy(params[1].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[2].0);

        self.renderer
            .draw_rectangle_mono(v, width, height, color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_vram_to_vram_blit(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let Vec2 { x: src_x, y: src_y } = parse_xy(params[1].0);
        let Vec2 { x: dst_x, y: dst_y } = parse_xy(params[2].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[3].0);

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

    pub fn gp0_poly_mono<const QUAD: bool, const TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_8bit(params[0].0);

        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[2].0);
        let v2 = parse_xy(params[3].0);

        self.renderer.draw_triangle(
            [v0, v1, v2],
            DrawOptions {
                color: ColorOptions::Mono(color),
                transparent: TRANS,
                textured: None,
            },
        );

        if QUAD {
            let v3 = parse_xy(params[4].0);
            self.renderer.draw_triangle(
                [v1, v2, v3],
                DrawOptions {
                    color: ColorOptions::Mono(color),
                    transparent: TRANS,
                    textured: None,
                },
            );
        }

        GP0State::AwaitCommand
    }

    pub fn gp0_poly_shaded<const QUAD: bool, const TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[3].0);
        let v2 = parse_xy(params[5].0);

        let c0 = Color::new_8bit(params[0].0);
        let c1 = Color::new_8bit(params[2].0);
        let c2 = Color::new_8bit(params[4].0);

        self.renderer.draw_triangle(
            [v0, v1, v2],
            DrawOptions {
                color: ColorOptions::Shaded([c0, c1, c2]),
                transparent: TRANS,
                textured: None,
            },
        );

        if QUAD {
            let v3 = parse_xy(params[7].0);
            let c3 = Color::new_8bit(params[6].0);
            self.renderer.draw_triangle(
                [v1, v2, v3],
                DrawOptions {
                    color: ColorOptions::Shaded([c1, c2, c3]),
                    transparent: TRANS,
                    textured: None,
                },
            );
        }

        GP0State::AwaitCommand
    }

    pub fn gp0_poly_texture<const QUAD: bool, const TRANS: bool, const BLEND: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_8bit(params[0].0);

        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[3].0);
        let v2 = parse_xy(params[5].0);

        let (clut, uv0) = parse_clut_uv(params[2].0);
        let (texture, uv1) = parse_page_uv(params[4].0, clut);
        let uv2 = parse_uv(params[6].0);

        self.renderer.draw_triangle(
            [v0, v1, v2],
            DrawOptions {
                color: ColorOptions::Mono(color),
                transparent: TRANS,
                textured: Some((texture, BLEND, [uv0, uv1, uv2])),
            },
        );

        if QUAD {
            let v3 = parse_xy(params[7].0);
            let uv3 = parse_uv(params[8].0);
            self.renderer.draw_triangle(
                [v1, v2, v3],
                DrawOptions {
                    color: ColorOptions::Mono(color),
                    transparent: TRANS,
                    textured: Some((texture, BLEND, [uv1, uv2, uv3])),
                },
            );
        }

        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_mono_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[2].0);
        let color = Color::new_8bit(params[0].0).to_5bit();

        self.renderer.draw_line_mono([v0, v1], color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_mono_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[2].0);
        let color = Color::new_8bit(params[0].0).to_5bit();

        self.renderer.draw_line_mono([v0, v1], color, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_shaded_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[3].0);

        let colors = [
            Color::new_8bit(params[0].0).to_5bit(),
            Color::new_8bit(params[2].0).to_5bit(),
        ];

        self.renderer.draw_line_shaded([v0, v1], colors, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_single_shaded_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[3].0);

        let colors = [
            Color::new_8bit(params[0].0).to_5bit(),
            Color::new_8bit(params[2].0).to_5bit(),
        ];

        self.renderer.draw_line_shaded([v0, v1], colors, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_mono_opaque(&mut self, vertices: Vec<u32>, colors: Vec<u32>) -> GP0State {
        let vertices = vertices.into_iter().map(parse_xy).collect();

        let color = Color::new_8bit(colors[0]).to_5bit();
        self.renderer.draw_line_poly_mono(vertices, color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_mono_trans(&mut self, vertices: Vec<u32>, colors: Vec<u32>) -> GP0State {
        let vertices = vertices.into_iter().map(parse_xy).collect();

        let color = Color::new_8bit(colors[0]).to_5bit();
        self.renderer.draw_line_poly_mono(vertices, color, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_shaded_opaque(
        &mut self,
        vertices: Vec<u32>,
        colors: Vec<u32>,
    ) -> GP0State {
        let vertices = vertices.into_iter().map(parse_xy).collect();

        let colors = colors
            .into_iter()
            .map(|word| Color::new_8bit(word).to_5bit())
            .collect();

        self.renderer.draw_line_poly_shaded(vertices, colors, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_line_poly_shaded_trans(&mut self, vertices: Vec<u32>, colors: Vec<u32>) -> GP0State {
        let vertices = vertices.into_iter().map(parse_xy).collect();

        let colors = colors
            .into_iter()
            .map(|word| Color::new_8bit(word).to_5bit())
            .collect();

        self.renderer.draw_line_poly_shaded(vertices, colors, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_1x1_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        self.renderer.draw_rectangle_mono(v, 1, 1, color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_1x1_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        self.renderer.draw_rectangle_mono(v, 1, 1, color, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_8x8_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        self.renderer.draw_rectangle_mono(v, 8, 8, color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_8x8_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        self.renderer.draw_rectangle_mono(v, 8, 8, color, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_16x16_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        self.renderer.draw_rectangle_mono(v, 16, 16, color, false);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_16x16_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        self.renderer.draw_rectangle_mono(v, 16, 16, color, true);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_var_opaque(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[2].0);

        self.renderer
            .draw_rectangle_mono(v, width, height, color, false);

        GP0State::AwaitCommand
    }

    pub fn gp0_rect_var_trans(&mut self, params: ArrayVec<Command, 16>) -> GP0State {
        let color = Color::new_8bit(params[0].0).to_5bit();
        let v = parse_xy(params[1].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[2].0);

        self.renderer
            .draw_rectangle_mono(v, width, height, color, true);

        GP0State::AwaitCommand
    }
}
