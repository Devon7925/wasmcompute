#![no_std]
#![no_main]
#[path = "../sdk/minecraft.rs"]
mod minecraft;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn run() -> i32 {
    minecraft::command_literal("scoreboard objectives add rustdemo dummy");
    minecraft::command_literal("scoreboard players set example rustdemo 42");
    minecraft::command_literal("scoreboard players get example rustdemo")
}
