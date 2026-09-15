#![no_std]
pub use minecraft_macros::*;

/// Execute a sequence of commands in order, discarding their results.
/// Each line uses the same literal/template syntax as [`command!`]. Arguments
/// are evaluated once when their command is reached; Rust control flow stays
/// outside the block. Use `command!` when you need a command's result.
///
/// ```ignore
/// minecraft::commands! {
///     "time set noon";
///     "setblock $(x) 80 64 minecraft:stone strict", x = column;
/// }
/// ```
#[macro_export]
macro_rules! commands {
    ($($text:literal $(, $name:ident $(: $ty:ident)? = $value:expr)* $(,)?);* $(;)?) => {{
        $( $crate::command!($text $(, $name $(: $ty)? = $value)*); )*
    }};
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandOutcome {
    pub result: i32,
    pub success: bool,
}
impl CommandOutcome {
    #[doc(hidden)]
    #[inline(always)]
    pub fn from_raw(raw: i64) -> Self {
        Self {
            result: raw as i32,
            success: (raw >> 32) != 0,
        }
    }
}

#[link(wasm_import_module = "minecraft")]
extern "C" {
    fn mc_command_utf8_result(ptr: *const u8, len: u32) -> i32;
}
/// Execute a command assembled at runtime. Prefer command! for literals/templates.
/// UTF-8, quotes and backslashes are preserved. Maximum length: 32,767 bytes.
/// Invalid buffers, UTF-8, line breaks and NUL fail before command execution.
pub fn command(text: &str) -> i32 {
    unsafe { mc_command_utf8_result(text.as_ptr(), text.len() as u32) }
}
