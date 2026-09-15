// Reuse the public Rust function implementations without command dispatch.
#[path = "data/celeste_direct/function/frame.rs"]
pub mod frame;
#[path = "data/celeste_direct/function/join.rs"]
pub mod join;
#[path = "data/celeste_direct/function/load.rs"]
pub mod load;
#[path = "data/celeste_direct/function/prepare.rs"]
pub mod prepare;
#[path = "data/celeste_direct/function/ready.rs"]
pub mod ready;
#[path = "data/celeste_direct/function/screen.rs"]
pub mod screen;
#[path = "data/celeste_direct/function/tick.rs"]
pub mod tick;
#[path = "data/celeste_direct/function/toggle.rs"]
pub mod toggle;
