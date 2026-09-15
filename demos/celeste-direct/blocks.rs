// Block output primitives for points, solid rectangles and immutable textures.
pub fn pixel(x: i32, y: i32, rgb: i32) {
    let y = 207 - y;
    match rgb {
        0 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:black_concrete strict",
                x = x,
                y = y
            );
        }
        1911635 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:blue_concrete strict",
                x = x,
                y = y
            );
        }
        8267091 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:magenta_concrete strict",
                x = x,
                y = y
            );
        }
        34641 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:green_concrete strict",
                x = x,
                y = y
            );
        }
        11227702 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:brown_concrete strict",
                x = x,
                y = y
            );
        }
        6248271 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:gray_concrete strict",
                x = x,
                y = y
            );
        }
        12764103 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:light_gray_concrete strict",
                x = x,
                y = y
            );
        }
        16773608 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:white_concrete strict",
                x = x,
                y = y
            );
        }
        16711757 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:red_concrete strict",
                x = x,
                y = y
            );
        }
        16753408 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:orange_concrete strict",
                x = x,
                y = y
            );
        }
        16772135 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:yellow_concrete strict",
                x = x,
                y = y
            );
        }
        58422 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:lime_concrete strict",
                x = x,
                y = y
            );
        }
        2731519 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:light_blue_concrete strict",
                x = x,
                y = y
            );
        }
        8615580 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:purple_concrete strict",
                x = x,
                y = y
            );
        }
        16742312 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:pink_concrete strict",
                x = x,
                y = y
            );
        }
        16764074 => {
            minecraft::command!(
                "setblock $(x) $(y) 64 minecraft:white_terracotta strict",
                x = x,
                y = y
            );
        }
        _ => {}
    }
}

pub fn rectangle(x: i32, y: i32, w: i32, h: i32, rgb: i32) {
    let x1 = x + w - 1;
    let y1 = 207 - y;
    let y = 208 - y - h;
    match rgb {
        0 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:black_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        1911635 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:blue_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        8267091 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:magenta_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        34641 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:green_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        11227702 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:brown_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        6248271 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:gray_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        12764103 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:light_gray_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        16773608 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:white_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        16711757 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:red_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        16753408 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:orange_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        16772135 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:yellow_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        58422 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:lime_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        2731519 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:light_blue_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        8615580 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:purple_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        16742312 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:pink_concrete strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        16764074 => {
            minecraft::command!(
                "fill $(x) $(y) 64 $(x1) $(y1) 64 minecraft:white_terracotta strict",
                x = x,
                y = y,
                x1 = x1,
                y1 = y1
            );
        }
        _ => {}
    }
}
pub fn atlas_pixel(x: i32, y: i32, z: i32, color: u8) {
    match color {
        0 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:black_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        1 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:blue_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        2 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:magenta_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        3 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:green_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        4 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:brown_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        5 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:gray_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        6 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:light_gray_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        7 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:white_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        8 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:red_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        9 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:orange_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        10 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:yellow_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        11 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:lime_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        12 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:light_blue_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        13 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:purple_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        14 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:pink_concrete strict",
                x = x,
                y = y,
                z = z
            );
        }
        15 => {
            minecraft::command!(
                "setblock $(x) $(y) $(z) minecraft:white_terracotta strict",
                x = x,
                y = y,
                z = z
            );
        }
        _ => {}
    }
}
