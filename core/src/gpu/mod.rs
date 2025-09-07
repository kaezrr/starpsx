mod commands;
mod utils;

use arrayvec::ArrayVec;
use starpsx_renderer::Renderer;
use utils::{
    DisplayDepth, DmaDirection, Field, GP0State, HorizontalRes, TextureDepth, VMode, VerticalRes,
};

use crate::gpu::utils::VramCopyFields;

bitfield::bitfield! {
    #[derive(Clone, Copy)]
    pub struct GpuStat(u32);
    page_base_x, set_page_base_x : 3, 0;
    page_base_y, set_page_base_y : 4;
    semi_transparency, set_semi_transparency : 6, 5;
    u8, from into TextureDepth, texture_depth, set_texture_depth : 8, 7;
    dithering, set_dithering : 9;
    draw_to_display, set_draw_to_display : 10;
    force_set_mask_bit, set_force_set_mask_bit : 11;
    preserve_masked_pixels, set_preserve_masked_pixels : 12;
    u8, from into Field, field, set_field : 13, 13;
    texture_disable, set_texture_disable : 15;
    u8, into HorizontalRes, hres, set_hres : 18, 16;
    u8, from into VerticalRes, vres, set_vres : 19, 19;
    u8, from into VMode, vmode, set_vmode : 20, 20;
    u8, from into DisplayDepth, display_depth, set_display_depth : 21, 21;
    interlaced, set_interlaced : 22;
    display_disabled, set_display_disabled : 23;
    interrupt, set_interrupt : 24;
    ready_cmd, set_ready_cmd : 26;
    ready_vram, set_ready_vram : 27;
    ready_dma_recv, set_ready_dma_recv: 28;
    u8, from into DmaDirection, dma_direction, set_dma_direction: 30, 29;
    even_odd_draw, set_even_odd_draw : 31;
}

bitfield::bitfield! {
    #[derive(Clone, Copy)]
    pub struct Command(u32);
    u8, opcode, _ : 31, 24;
    register_index, _ : 23, 0;

    //GP0 Draw Mode
    page_base_x, _ : 3, 0;
    page_base_y, _ : 4;
    semi_transparency, _ : 6, 5;
    u8, into TextureDepth, texture_depth, _ : 8, 7;
    dithering, _ : 9;
    draw_to_display, _ : 10;
    texture_disable, _ : 11;
    texture_rect_x_flip, _ : 12;
    texture_rect_y_flip, _ : 13;

    // GP1 Display Mode
    u8, hres_1, _ : 1, 0;
    u8, hres_2, _ : 6, 6;
    u8, into VerticalRes, vres, _ : 2, 2;
    u8, into VMode, vmode, _ : 3, 3;
    u8, into DisplayDepth, display_depth, _ : 4, 4;
    interlaced, _ : 5;
    flip_screen, _ : 7;

    //GP1 DMA Direction
    u8, into DmaDirection, dma_direction, _ : 1, 0;

    //GP0 Set Drawing Area
    u16, x_coordinates, _ : 9, 0;
    u16, y_coordinates, _ : 19, 10;

    //GP0 Set Drawing Offset
    u16, x_offset, _ : 10, 0;
    u16, y_offset, _ : 21, 11;

    //GP0 Set Texture Window
    u8, window_mask_x, _: 4, 0;
    u8, window_mask_y, _: 9, 5;
    u8, window_offset_x, _: 14, 10;
    u8, window_offset_y, _: 19, 15;

    // GP0 Set Mask Bit Setting
    force_set_mask_bit, _: 0;
    preserve_masked_pixels, _: 1;

    // GP1 Display VRAM Start
    u16, display_vram_x, _ : 9, 1; // LSB of horizontal component is ignored?
    u16, display_vram_y, _ : 18, 10;

    // GP1 Display Horizontal and Vertical Ranges
    u16, horizontal_x1, _ : 11, 0;
    u16, horizontal_x2, _ : 23, 12;
    u16, vertical_y1, _ : 9, 0;
    u16, vertical_y2, _ : 19, 10;

    // GP0 Load Image
    image_width, _ : 15, 0;
    image_height, _ : 31, 16;

    // GP1 Display Enable
    display_off, _ : 0;
}

pub struct Gpu {
    pub renderer: Renderer,
    gpu_read: u32,
    stat: GpuStat,
    gp0_params: ArrayVec<Command, 16>,
    gp0_state: GP0State,
}

impl Default for Gpu {
    fn default() -> Self {
        Self {
            renderer: Renderer::default(),
            gpu_read: 0,
            stat: GpuStat(0),
            gp0_params: ArrayVec::new(),
            gp0_state: GP0State::AwaitCommand,
        }
    }
}

impl Gpu {
    pub fn stat(&self) -> u32 {
        let mut ret = self.stat;
        // GPU always ready for commands, for now
        ret.set_ready_cmd(true);
        ret.set_ready_vram(true);
        ret.set_ready_dma_recv(true);

        // Hack, GPU doesn't have proper timing to emulate vres 480 lines
        ret.set_vres(VerticalRes::Y240);
        ret.0
    }

    pub fn read(&mut self) -> u32 {
        if let GP0State::CopyFromVram(fields) = self.gp0_state {
            self.process_vram_to_cpu_copy(fields);
        }
        self.gpu_read
    }

    pub fn gp0(&mut self, data: u32) {
        match self.gp0_state {
            GP0State::AwaitCommand => self.process_command(data),
            GP0State::AwaitArgs { cmd, len } => self.process_argument(data, cmd, len),
            GP0State::CopyToVram(x) => self.process_cpu_to_vram_copy(data, x),
            GP0State::CopyFromVram(_) => panic!("VRAM currently being copying to CPU!"),
        };
    }

    pub fn gp1(&mut self, data: u32) {
        let command = Command(data);
        match command.opcode() {
            0x00 => self.gp1_reset(),
            0x01 => self.gp1_reset_command_buffer(),
            0x02 => self.gp1_acknowledge_irq(),
            0x03 => self.gp1_display_enable(command),
            0x04 => self.gp1_dma_direction(command),
            0x05 => self.gp1_display_vram_start(command),
            0x06 => self.gp1_display_horizontal_range(command),
            0x07 => self.gp1_display_vertical_range(command),
            0x08 => self.gp1_display_mode(command),
            0x10 => self.gp1_read_internal_reg(command),
            _ => panic!("Unknown GP1 command {data:08x}"),
        }
    }

    fn process_cpu_to_vram_copy(&mut self, word: u32, mut fields: VramCopyFields) {
        for i in 0..2 {
            let halfword = (word >> (16 * i)) as u16;
            let vram_row = ((fields.vram_y + fields.current_row) & 0x1FF) as usize;
            let vram_col = ((fields.vram_x + fields.current_col) & 0x3FF) as usize;
            self.renderer.vram_write(vram_col, vram_row, halfword);

            fields.current_col += 1;
            if fields.current_col == fields.width {
                fields.current_col = 0;
                fields.current_row += 1;

                if fields.current_row == fields.height {
                    return self.gp0_state = GP0State::AwaitCommand;
                }
            }
        }
        self.gp0_state = GP0State::CopyToVram(fields);
    }

    fn process_vram_to_cpu_copy(&mut self, mut fields: VramCopyFields) {
        let vram_row = ((fields.vram_y + fields.current_row) & 0x1FF) as usize;
        let vram_col = ((fields.vram_x + fields.current_col) & 0x3FF) as usize;

        let lo: u32 = self.renderer.vram_read(vram_col, vram_row).into();
        let hi: u32 = self.renderer.vram_read(vram_col + 1, vram_row).into();

        let data = lo | (hi << 16);

        fields.current_col += 2;
        if fields.current_col >= fields.width {
            fields.current_col = 0;
            fields.current_row += 1;

            if fields.current_row == fields.height {
                return self.gp0_state = GP0State::AwaitCommand;
            }
        }
        self.gp0_state = GP0State::CopyFromVram(fields);
        self.gpu_read = data;
    }

    fn process_argument(&mut self, data: u32, cmd: fn(&mut Gpu), len: usize) {
        let command = Command(data);
        self.gp0_params.push(command);
        if self.gp0_params.len() == len {
            self.gp0_state = GP0State::AwaitCommand;
            (cmd)(self);
            self.gp0_params.clear();
        }
    }

    #[inline(always)]
    fn process_command(&mut self, data: u32) {
        let command = Command(data);
        let (len, cmd): (usize, fn(&mut Gpu)) = match command.opcode() {
            0x00 => (1, Gpu::gp0_nop),
            0x01 => (1, Gpu::gp0_clear_cache),
            0x02 => (3, Gpu::gp0_quick_rect_fill),
            0x28 => (5, Gpu::gp0_quad_mono_opaque),
            0x2C => (9, Gpu::gp0_quad_texture_blend_opaque),
            0x30 => (6, Gpu::gp0_triangle_shaded_opaque),
            0x38 => (8, Gpu::gp0_quad_shaded_opaque),
            0x68 => (2, Gpu::gp0_draw_1x1_rectangle),
            0x80 => (4, Gpu::gp0_vram_to_vram_blit),
            0xA0 => (3, Gpu::gp0_image_load),
            0xC0 => (3, Gpu::gp0_image_store),
            0xE1 => (1, Gpu::gp0_draw_mode),
            0xE2 => (1, Gpu::gp0_texture_window),
            0xE3 => (1, Gpu::gp0_drawing_area_top_left),
            0xE4 => (1, Gpu::gp0_drawing_area_bottom_right),
            0xE5 => (1, Gpu::gp0_drawing_area_offset),
            0xE6 => (1, Gpu::gp0_mask_bit_setting),
            _ => panic!("Unknown GP0 command {data:08x}"),
        };
        self.gp0_state = GP0State::AwaitArgs { cmd, len };
        self.process_argument(data, cmd, len);
    }

    pub fn get_resolution(&self) -> (usize, usize) {
        // let width = match self.stat.hres() {
        //     HorizontalRes::X256 => 256,
        //     HorizontalRes::X320 => 320,
        //     HorizontalRes::X512 => 512,
        //     HorizontalRes::X368 => 368,
        //     HorizontalRes::X640 => 640,
        // };
        //
        // let height = match self.stat.vres() {
        //     VerticalRes::Y240 => 240,
        //     VerticalRes::Y480 => 480,
        // };
        //
        // (width, height)
        (1024, 512)
    }
}
