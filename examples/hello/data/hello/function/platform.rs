// SPDX-License-Identifier: 0BSD
pub fn main() -> i32 {
    let width = 5;
    let height = 2;
    minecraft::commands! {
        "say Building a platform from Rust!";
        "fill ~ ~-1 ~ ~$(width) ~-1 ~$(width) minecraft:stone strict", width = width;
    }
    minecraft::command!(
        "setblock ~2 ~$(height) ~2 minecraft:sea_lantern strict",
        height = height
    )
}
