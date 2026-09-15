//! Immutable block textures. Startup cost is paid once per datapack load.
const GFX: &[u8; 8192] = include_bytes!(concat!(env!("OUT_DIR"), "/gfx.bin"));
const FONT: &[u8; 8192] = include_bytes!(concat!(env!("OUT_DIR"), "/font.bin"));
const IDENTITY: [u8;16] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15];
static mut REMAP: [u8;16] = IDENTITY;
static mut CHANGED: u32 = 0;
pub fn palette(a:i32,b:i32) {
    if !(0..16).contains(&a) || !(0..16).contains(&b) {return;}
    unsafe {REMAP[a as usize]=b as u8; if a==b {CHANGED &= !(1<<a);} else {CHANGED |= 1<<a;}}
}
pub fn reset_palette() {unsafe {if CHANGED!=0 {REMAP=IDENTITY;CHANGED=0;}}}
pub fn prepare() {
    for layer in 0..24i32 {
        let z = 64 + layer;
        minecraft::command!("fill 256 80 $(z) 383 143 $(z) minecraft:air strict", z = z);
        for y in 0..64 {
            for x in 0..128 {
                let source_x = if layer == 1 || (layer>=18 && layer%2==1) { x / 8 * 8 + 7 - x % 8 } else { x };
                let index = if layer < 2 || layer>=18 {
                    GFX[y * 128 + source_x]
                } else {
                    FONT[y * 128 + x]
                };
                if index != 0 {
                    let color = if layer<2 {index} else if layer>=18 {if index==8 {[7,11,12][((layer-18)/2) as usize]} else {index}} else {(layer-2) as u8};
                    crate::blocks::atlas_pixel(256 + x as i32, 143 - y as i32, z, color);
                }
            }
        }
    }
}
pub fn blit(
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
    let changed=unsafe {CHANGED};
    let mapped_color=unsafe {REMAP[(color as u8%16) as usize]} as i32;
    let sprite_layer=if changed==0 {0} else if changed==1<<8 {
        match unsafe {REMAP[8]} {7=>18,11=>20,12=>22,_=>return 0}
    } else {return 0;};
    if sx < 0
        || sy < 0
        || sx + 8 > 128
        || sy + 8 > 64
        || sx % 8 != 0
        || sy % 8 != 0
        || w > 8
        || h > 8
        || (asset == 1 && flip != 0)
        || (asset == 0 && color != 0)
    {
        return 0;
    }
    let w = w.min(128 - x);
    let h = h.min(128 - y);
    if w == 0 || h == 0 {
        return 1;
    }
    // The upstream masked blitter also paints its top-left pixel unconditionally.
    if color != 0 {
        const PALETTE: [i32; 16] = [
            0, 0x1d2b53, 0x7e2553, 0x008751, 0xab5236, 0x5f574f, 0xc2c3c7, 0xfff1e8, 0xff004d,
            0xffa300, 0xffec27, 0x00e436, 0x29adff, 0x83769c, 0xff77a8, 0xffccaa,
        ];
        crate::direct_pixel(x, y, PALETTE[mapped_color as usize]);
    }
    if w < 0 || h < 0 {
        return 1;
    }
    let left = (-x).max(0);
    let top = (-y).max(0);
    if left >= w || top >= h {
        return 1;
    }
    let layer = if asset == 0 {
        sprite_layer + if flip != 0 {1} else {0}
    } else {
        2 + if color == 0 {
            1
        } else {
            mapped_color
        }
    };
    let sx = 256 + sx + if flip != 0 { 8 - w } else { 0 };
    let x1 = sx + left;
    let x2 = sx + w - 1;
    let y1 = 144 - sy - h;
    let y2 = 143 - sy - top;
    let z = 64 + layer;
    let target = unsafe { TARGET };
    if target >= 0 {
        let bounds = [x + left, y + top, x + w - 1, y + h - 1];
        unsafe {
            BOUNDS[target as usize] = Some(match BOUNDS[target as usize] {
                None => bounds,
                Some(old) => [
                    old[0].min(bounds[0]),
                    old[1].min(bounds[1]),
                    old[2].max(bounds[2]),
                    old[3].max(bounds[3]),
                ],
            });
        }
    }
    let dx = x + left + if target >= 0 { 512 } else { 0 };
    let dy = 208 - y - h;
    let dz = 64 + target.max(0);
    minecraft::command!(
        "clone $(x1) $(y1) $(z) $(x2) $(y2) $(z) $(dx) $(dy) $(dz) strict masked force",
        x1 = x1,
        y1 = y1,
        z = z,
        x2 = x2,
        y2 = y2,
        dx = dx,
        dy = dy,
        dz = dz
    );
    unsafe {
        crate::WRITES += 1;
    }
    1
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct MapKey {
    mx: i32,
    my: i32,
    mw: i32,
    mh: i32,
    mask: i32,
}
static mut KEYS: [Option<MapKey>; 3] = [None; 3];
static mut TARGET: i32 = -1;
static mut SLOT: i32 = 0;
static mut BOUNDS: [Option<[i32; 4]>; 3] = [None; 3];
pub fn map_begin(mx: i32, my: i32, mw: i32, mh: i32, mask: i32) -> i32 {
    if unsafe {CHANGED}!=0 {return -1;}
    let slot = match mask {
        4 => 0,
        2 => 1,
        8 => 2,
        _ => return -1,
    };
    if mw != 16 || mh != 16 {
        return -1;
    }
    let key = MapKey {
        mx,
        my,
        mw,
        mh,
        mask,
    };
    unsafe {
        SLOT = slot;
        if KEYS[slot as usize] == Some(key) {
            return 0;
        }
        KEYS[slot as usize] = Some(key);
        TARGET = slot;
        BOUNDS[slot as usize] = None;
    }
    let z = 64 + slot;
    minecraft::command!("fill 512 80 $(z) 639 207 $(z) minecraft:air strict", z = z);
    unsafe {
        crate::WRITES += 1;
    }
    1
}
pub fn map_end(x: i32, y: i32) {
    let slot = unsafe {
        TARGET = -1;
        SLOT
    };
    let Some(bounds) = (unsafe { BOUNDS[slot as usize] }) else {
        return;
    };
    let left = bounds[0].max(-x);
    let top = bounds[1].max(-y);
    let right = bounds[2].min(127 - x);
    let bottom = bounds[3].min(127 - y);
    if left > right || top > bottom {
        return;
    }
    let x1 = 512 + left;
    let x2 = 512 + right;
    let y1 = 207 - bottom;
    let y2 = 207 - top;
    let z = 64 + slot;
    let dx = x + left;
    let dy = 207 - y - bottom;
    minecraft::command!(
        "clone $(x1) $(y1) $(z) $(x2) $(y2) $(z) $(dx) $(dy) 64 strict masked force",
        x1 = x1,
        y1 = y1,
        z = z,
        x2 = x2,
        y2 = y2,
        dx = dx,
        dy = dy
    );
    unsafe {
        crate::WRITES += 1;
    }
}
