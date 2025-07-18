mod utils;

use bitfield::bitfield;
use utils::{DisplayDepth, DmaDirection, Field, TextureDepth, VMode, VerticalRes};

bitfield! {
    pub struct GpuStat(u32);
    u32;
    page_base_x, _ : 3, 0;
    page_base_y, _ : 4;
    semi_transparency, _ : 6, 5;
    u8, into TextureDepth, texture_depth, _ : 8, 7;
    dithering, _ : 9;
    draw_to_display, _ : 10;
    force_set_mask_bit, _ : 11;
    preserve_masked_pixels, _ : 12;
    u8, into Field, field, _ : 13, 13;
    texture_disable, _ : 15;
    hres, _ : 18, 16;
    u8, into VerticalRes, vres, _ : 19, 19;
    u8, into VMode, vmode, _ : 20, 20;
    u8, into DisplayDepth, display_depth, _ : 21, 21;
    interlaced, _ : 22;
    display_disabled, _ : 23;
    interrupt, _ : 24;
    ready_cmd, _ : 26;
    ready_vram, _ : 27;
    ready_dma_recv, _ : 28;
    u8, into DmaDirection, dma_direction, _: 29, 30;
    even_odd_draw, _: 31;
}

pub struct Gpu {
    stat: GpuStat,
}
