use bitfield::bitfield;

bitfield! {
    pub struct GpuStat(u32);
    u32;
    page_base_x, _ : 3, 0;
    page_base_y, _ : 4;
    semi_transparency, _ : 6, 5;
    texture_depth, _ : 8, 7;
    dithering, _ : 9;
    draw_to_display, _ : 10;
    force_set_mask_bit, _ : 11;
    preserve_masked_pixels, _ : 12;
    field, _ : 13;
    texture_disable, _ : 14;
    hres, _ : 18, 16;
    vres, _ : 19;
    vmode, _ : 20;
    display_depth, _ : 21;
    interlaced, _ : 22;
    display_disabled, _ : 23;
    interrupt, _ : 24;
    dma_direction, _: 29, 30;
}

pub struct Gpu {
    stat: GpuStat,
}
