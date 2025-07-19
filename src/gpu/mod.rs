mod commands;
mod utils;

use utils::{DisplayDepth, DmaDirection, Field, TextureDepth, VMode, VerticalRes};

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
    hres, set_hres : 18, 16;
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
    hres_1, _ : 1, 0;
    hres_2, _ : 6, 6;
    u8, into VerticalRes, vres, _ : 2, 2;
    u8, into VMode, vmode, _ : 3, 3;
    u8, into DisplayDepth, display_depth, _ : 4, 4;
    interlaced, _ : 5;
    flip_screen, _ : 7;

    //GP1 DMA Direction
    u8, into DmaDirection, dma_direction, _ : 1, 0;
}

pub struct Gpu {
    stat: GpuStat,
    texture_rect_x_flip: bool,
    texture_rect_y_flip: bool,

    texture_window_x_mask: u8,
    texture_window_y_mask: u8,

    texture_window_x_offset: u8,
    texture_window_y_offset: u8,

    drawing_area_left: u16,
    drawing_area_top: u16,
    drawing_area_right: u16,
    drawing_area_bottom: u16,

    drawing_x_offset: i16,
    drawing_y_offset: i16,

    display_vram_x_start: u16,
    display_vram_y_start: u16,

    display_hori_start: u16,
    display_hori_end: u16,

    display_line_start: u16,
    display_line_end: u16,
}

impl Gpu {
    pub fn new() -> Self {
        Gpu {
            stat: GpuStat(0),

            texture_rect_x_flip: false,
            texture_rect_y_flip: false,

            texture_window_x_mask: 0,
            texture_window_y_mask: 0,

            texture_window_x_offset: 0,
            texture_window_y_offset: 0,

            drawing_area_left: 0,
            drawing_area_top: 0,
            drawing_area_right: 0,
            drawing_area_bottom: 0,

            drawing_x_offset: 0,
            drawing_y_offset: 0,

            display_vram_x_start: 0,
            display_vram_y_start: 0,

            display_hori_start: 0,
            display_hori_end: 0,

            display_line_start: 0,
            display_line_end: 0,
        }
    }

    pub fn stat(&self) -> u32 {
        let mut ret = self.stat;
        // GPU always ready for commands, for now
        ret.set_ready_cmd(true);
        ret.set_ready_vram(true);
        ret.set_ready_dma_recv(true);
        ret.0
    }

    pub fn read(&self) -> u32 {
        0
    }

    pub fn gp0(&mut self, data: u32) {
        let command = Command(data);

        match command.opcode() {
            0x00 => (), // NOP
            0xE1 => self.gp0_draw_mode(command),
            _ => panic!("Unknown GP0 command {data:08x}"),
        }
    }

    pub fn gp1(&mut self, data: u32) {
        let command = Command(data);

        match command.opcode() {
            0x00 => self.gp1_reset(),
            0x04 => self.gp1_dma_direction(command),
            0x08 => self.gp1_display_mode(command),
            _ => panic!("Unknown GP1 command {data:08x}"),
        }
    }
}
