use super::utils::{parse_clut_uv, parse_page_uv, parse_uv, parse_xy};
use super::*;
use starpsx_renderer::utils::Texture;
use starpsx_renderer::{
    utils::{Color, ColorOptions, DrawOptions},
    vec2::Vec2,
};

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
        let cmd = params[0];

        self.gpu_stat.set_page_base_x(cmd.page_base_x());
        self.gpu_stat.set_page_base_y(cmd.page_base_y());
        self.gpu_stat.set_semi_transparency(cmd.semi_transparency());
        self.gpu_stat.set_texture_depth(cmd.texture_depth());
        self.gpu_stat.set_dithering(cmd.dithering());
        self.gpu_stat.set_draw_to_display(cmd.draw_to_display());
        self.gpu_stat.set_texture_disable(cmd.texture_disable());

        let ctx = &mut self.renderer.ctx;

        ctx.rect_texture = Texture::new(cmd.0 as u16, None);
        ctx.dithering = self.gpu_stat.dithering();
        ctx.transparency_weights = match self.gpu_stat.semi_transparency() {
            0 => (0.5, 0.5),
            1 => (1.0, 1.0),
            2 => (1.0, -1.0),
            3 => (1.0, 0.25),
            _ => unreachable!("2 bit value cant reach here"),
        };

        ctx.texture_flip_x = cmd.texture_rect_x_flip();
        ctx.texture_flip_y = cmd.texture_rect_y_flip();

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
        self.gpu_stat
            .set_force_set_mask_bit(params[0].force_set_mask_bit());
        self.gpu_stat
            .set_preserve_masked_pixels(params[0].preserve_masked_pixels());
        self.renderer.ctx.force_set_masked_bit = params[0].force_set_mask_bit();
        self.renderer.ctx.preserve_masked_pixels = params[0].preserve_masked_pixels();
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
        let color = Color::new_8bit(params[0].0);
        let v = parse_xy(params[1].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[2].0);

        self.renderer.vram_quick_fill(v, width, height, color);
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

    pub fn gp0_poly_mono<const QUAD: bool, const SEMI_TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_5bit(params[0].0);

        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[2].0);
        let v2 = parse_xy(params[3].0);

        self.renderer.draw_triangle(
            [v0, v1, v2],
            DrawOptions {
                color: ColorOptions::Mono(color),
                transparent: SEMI_TRANS,
                textured: None,
            },
        );

        if QUAD {
            let v3 = parse_xy(params[4].0);
            self.renderer.draw_triangle(
                [v1, v2, v3],
                DrawOptions {
                    color: ColorOptions::Mono(color),
                    transparent: SEMI_TRANS,
                    textured: None,
                },
            );
        }

        GP0State::AwaitCommand
    }

    pub fn gp0_poly_shaded<const QUAD: bool, const SEMI_TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[3].0);
        let v2 = parse_xy(params[5].0);

        let c0 = Color::new_5bit(params[0].0);
        let c1 = Color::new_5bit(params[2].0);
        let c2 = Color::new_5bit(params[4].0);

        self.renderer.draw_triangle(
            [v0, v1, v2],
            DrawOptions {
                color: ColorOptions::Shaded([c0, c1, c2]),
                transparent: SEMI_TRANS,
                textured: None,
            },
        );

        if QUAD {
            let v3 = parse_xy(params[7].0);
            let c3 = Color::new_5bit(params[6].0);
            self.renderer.draw_triangle(
                [v1, v2, v3],
                DrawOptions {
                    color: ColorOptions::Shaded([c1, c2, c3]),
                    transparent: SEMI_TRANS,
                    textured: None,
                },
            );
        }

        GP0State::AwaitCommand
    }

    pub fn gp0_poly_texture<const QUAD: bool, const SEMI_TRANS: bool, const BLEND: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_5bit(params[0].0);

        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[3].0);
        let v2 = parse_xy(params[5].0);

        let (clut, uv0) = parse_clut_uv(params[2].0);
        let (texture, uv1) = parse_page_uv(params[4].0, clut);
        let uv2 = parse_uv(params[6].0);

        // for some goddamn reason this also updates the global texture
        self.renderer.ctx.rect_texture = texture;

        self.renderer.draw_triangle(
            [v0, v1, v2],
            DrawOptions {
                color: ColorOptions::Mono(color),
                transparent: SEMI_TRANS,
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
                    transparent: SEMI_TRANS,
                    textured: Some((texture, BLEND, [uv1, uv2, uv3])),
                },
            );
        }

        GP0State::AwaitCommand
    }

    pub fn gp0_poly_texture_shaded<const QUAD: bool, const SEMI_TRANS: bool, const BLEND: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let c0 = Color::new_5bit(params[0].0);
        let c1 = Color::new_5bit(params[3].0);
        let c2 = Color::new_5bit(params[6].0);

        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[4].0);
        let v2 = parse_xy(params[7].0);

        let (clut, uv0) = parse_clut_uv(params[2].0);
        let (texture, uv1) = parse_page_uv(params[5].0, clut);
        let uv2 = parse_uv(params[8].0);

        self.renderer.draw_triangle(
            [v0, v1, v2],
            DrawOptions {
                color: ColorOptions::Shaded([c0, c1, c2]),
                transparent: SEMI_TRANS,
                textured: Some((texture, BLEND, [uv0, uv1, uv2])),
            },
        );

        if QUAD {
            let c3 = Color::new_5bit(params[9].0);
            let v3 = parse_xy(params[10].0);
            let uv3 = parse_uv(params[11].0);
            self.renderer.draw_triangle(
                [v1, v2, v3],
                DrawOptions {
                    color: ColorOptions::Shaded([c1, c2, c3]),
                    transparent: SEMI_TRANS,
                    textured: Some((texture, BLEND, [uv1, uv2, uv3])),
                },
            );
        }

        GP0State::AwaitCommand
    }

    pub fn gp0_line_mono<const SEMI_TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[2].0);
        let color = Color::new_5bit(params[0].0);

        self.renderer.draw_line(
            [v0, v1],
            DrawOptions {
                color: ColorOptions::Mono(color),
                transparent: SEMI_TRANS,
                textured: None,
            },
        );
        GP0State::AwaitCommand
    }

    pub fn gp0_line_shaded<const SEMI_TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let v0 = parse_xy(params[1].0);
        let v1 = parse_xy(params[3].0);

        let c0 = Color::new_5bit(params[0].0);
        let c1 = Color::new_5bit(params[2].0);

        self.renderer.draw_line(
            [v0, v1],
            DrawOptions {
                color: ColorOptions::Shaded([c0, c1]),
                transparent: SEMI_TRANS,
                textured: None,
            },
        );
        GP0State::AwaitCommand
    }

    pub fn gp0_line_mono_poly<const SEMI_TRANS: bool>(
        &mut self,
        vertices: Vec<u32>,
        colors: Vec<u32>,
    ) -> GP0State {
        let vertices: Vec<Vec2> = vertices.into_iter().map(parse_xy).collect();
        let color = Color::new_5bit(colors[0]);

        for i in 1..vertices.len() {
            self.renderer.draw_line(
                [vertices[i - 1], vertices[i]],
                DrawOptions {
                    color: ColorOptions::Mono(color),
                    transparent: SEMI_TRANS,
                    textured: None,
                },
            );
        }
        GP0State::AwaitCommand
    }

    pub fn gp0_line_shaded_poly<const SEMI_TRANS: bool>(
        &mut self,
        vertices: Vec<u32>,
        colors: Vec<u32>,
    ) -> GP0State {
        let vertices: Vec<Vec2> = vertices.into_iter().map(parse_xy).collect();
        let colors: Vec<Color> = colors.into_iter().map(Color::new_5bit).collect();

        for i in 1..vertices.len() {
            self.renderer.draw_line(
                [vertices[i - 1], vertices[i]],
                DrawOptions {
                    color: ColorOptions::Shaded([colors[i - 1], colors[i]]),
                    transparent: SEMI_TRANS,
                    textured: None,
                },
            );
        }
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_fixed<const SIDE: i32, const SEMI_TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_8bit(params[0].0);
        let v = parse_xy(params[1].0);
        self.renderer
            .draw_rectangle_mono::<SEMI_TRANS>(v, SIDE, SIDE, color, None);
        GP0State::AwaitCommand
    }

    pub fn gp0_rect_texture_fixed<const SIDE: i32, const SEMI_TRANS: bool, const BLEND: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_8bit(params[0].0);
        let v = parse_xy(params[1].0);
        let (clut, uv) = parse_clut_uv(params[2].0);

        self.renderer.draw_rectangle_mono::<SEMI_TRANS>(
            v,
            SIDE,
            SIDE,
            color,
            Some((clut, BLEND, uv)),
        );

        GP0State::AwaitCommand
    }

    pub fn gp0_rect_variable<const SEMI_TRANS: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_8bit(params[0].0);
        let v = parse_xy(params[1].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[2].0);

        self.renderer
            .draw_rectangle_mono::<SEMI_TRANS>(v, width, height, color, None);

        GP0State::AwaitCommand
    }

    pub fn gp0_rect_texture_variable<const SEMI_TRANS: bool, const BLEND: bool>(
        &mut self,
        params: ArrayVec<Command, 16>,
    ) -> GP0State {
        let color = Color::new_8bit(params[0].0);
        let v = parse_xy(params[1].0);
        let (clut, uv) = parse_clut_uv(params[2].0);
        let Vec2 {
            x: width,
            y: height,
        } = parse_xy(params[3].0);

        self.renderer.draw_rectangle_mono::<SEMI_TRANS>(
            v,
            width,
            height,
            color,
            Some((clut, BLEND, uv)),
        );

        GP0State::AwaitCommand
    }
}
