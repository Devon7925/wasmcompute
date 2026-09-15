pub fn ensure_screen() {
    if !crate::state::get().screen_ready {
        if crate::BLOCKS
            || minecraft::command!("execute if entity @e[tag=celeste_direct_pixel,limit=1]") == 0
        {
            crate::lifecycle::screen::main();
        }
        crate::state::update(|s| s.screen_ready = true);
    }
}
