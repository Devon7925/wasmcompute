use celeste_direct as demo;
pub fn main() -> i32 {
    let mut loaded = true;
    if demo::BLOCKS {
        // Wait for every cache/screen chunk, not just the corners of each area.
        for start in [0, 256, 512] {
            for x in (start..start + 128).step_by(16) {
                loaded &= minecraft::command!("execute if loaded $(x) 80 64", x = x) != 0;
                if start == 256 {
                    loaded &= minecraft::command!("execute if loaded $(x) 80 80", x = x) != 0;
                }
            }
        }
        for x in [54, 74] {
            for z in [152, 170] {
                loaded &= minecraft::command!("execute if loaded $(x) 141 $(z)", x = x, z = z) != 0;
            }
        }
    } else {
        loaded = minecraft::command!(
            "execute if loaded -10 68 8 if loaded 10 68 22 if loaded -8 63 -1 if loaded 8 77 -1"
        ) != 0;
    }
    if !loaded {
        minecraft::command!("schedule function celeste_direct:ready 2t replace");
        return 0;
    }
    if demo::BLOCKS {
        minecraft::commands! {
            "fill 54 141 152 74 141 170 minecraft:black_concrete strict";
            "fill 0 80 63 127 207 63 minecraft:black_concrete strict";
        }
    } else {
        minecraft::commands! {
            "fill -10 68 8 10 68 22 minecraft:black_concrete";
            "fill -8 63 -1 8 77 -1 minecraft:black_concrete";
        }
    }
    demo::reset();
    demo::state::update(|s| {
        s.phase = demo::state::Phase::Ready;
        s.running = true;
    });
    0
}
