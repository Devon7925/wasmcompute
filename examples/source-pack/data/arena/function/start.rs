// An advancement or execute-as caller supplies @s and the command context.
use minecraft::command;

pub fn main() -> i32 {
    command!(r#"tellraw @s {"text":"Starting arena example"}"#);
    command!("function arena:paint")
}
