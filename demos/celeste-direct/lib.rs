#![no_std]
extern "C" {
    fn play_reset();
    fn play_frame(input: u32);
}
// Requested output operations: a pixel, solid fill, or texture copy counts once.
static mut WRITES: i32 = 0;
#[no_mangle]
pub extern "C" fn direct_pixel(x: i32, y: i32, rgb: i32) {
    if (x as u32) >= 128 || (y as u32) >= 128 {
        return;
    }
    let id = 10_000_001 + y * 128 + x;
    let color = rgb | 0xff000000u32 as i32;
    if BLOCKS {
        blocks::pixel(x, y, rgb);
    } else {
        minecraft::command!(
            "data modify entity 43455044-4952-4543-0000-0000$(id) background set value $(color)",
            id = id,
            color = color
        );
    }
    unsafe {
        WRITES += 1;
    }
}
pub fn reset() -> i32 {
    unsafe {
        renderer_support::ensure_screen();
        WRITES = 0;
        play_reset();
        WRITES
    }
}
pub fn step() -> i32 {
    let input = state::get().input;
    unsafe {
        WRITES = 0;
        play_frame(input as u32);
        WRITES
    }
}

extern crate self as celeste_direct;
pub mod lifecycle;
pub const IS_DIRECT: bool = true;
pub const BLOCKS: bool = cfg!(celeste_blocks);
mod blocks;

mod renderer_support;

pub mod state;

/// Platform input hook, also used for deterministic headless replays.
#[no_mangle]
pub extern "C" fn celeste_set_input(buttons: u32) {
    state::update(|s| s.input = buttons);
}

pub const ATLAS: bool = cfg!(celeste_atlas);
mod atlas;
/// A solid rectangle is a single draw operation on the block backend.
#[no_mangle]
pub extern "C" fn direct_rect(x: i32, y: i32, w: i32, h: i32, rgb: i32) {
    if BLOCKS {
        blocks::rectangle(x, y, w, h, rgb);
        unsafe {
            WRITES += 1;
        }
    } else {
        for yy in y..y + h {
            for xx in x..x + w {
                direct_pixel(xx, yy, rgb);
            }
        }
    }
}
#[no_mangle]
pub extern "C" fn direct_blit(
    asset: i32,
    sx: i32,
    sy: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: i32,
    flip: i32,
) -> i32 {
    if BLOCKS && ATLAS {
        atlas::blit(asset, sx, sy, x, y, w, h, color, flip)
    } else {
        0
    }
}

pub fn prepare_atlas() {
    atlas::prepare();
}

#[no_mangle]
pub extern "C" fn direct_map_begin(mx: i32, my: i32, mw: i32, mh: i32, mask: i32) -> i32 {
    if BLOCKS && cfg!(celeste_map_cache) {
        atlas::map_begin(mx, my, mw, mh, mask)
    } else {
        -1
    }
}
#[no_mangle]
pub extern "C" fn direct_map_end(x: i32, y: i32) {
    atlas::map_end(x, y);
}

#[no_mangle]
pub extern "C" fn direct_palette(a:i32,b:i32) {atlas::palette(a,b);}
#[no_mangle]
pub extern "C" fn direct_palette_reset() {atlas::reset_palette();}
