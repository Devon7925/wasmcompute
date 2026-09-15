//! Copy this small freestanding interface into a wasm32 Rust program.
#[link(wasm_import_module = "minecraft")]
extern "C" {
    pub fn mc_command_result(ptr:*const u8,len:u32)->i32;
    pub fn mc_command_success(ptr:*const u8,len:u32)->i32;
    pub fn mc_command_static_result(ptr:*const u8,len:u32)->i32;
    pub fn mc_command_static_success(ptr:*const u8,len:u32)->i32;
}
/// The compiler requires a constant pointer and length to immutable data.
#[inline(always)]
pub fn command_literal(command:&'static str)->i32 {
    unsafe {mc_command_static_result(command.as_ptr(),command.len() as u32)}
}
