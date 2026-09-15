use celeste_direct as demo;

pub fn main() -> i32 {
    let mut buttons = 0u32;
    if minecraft::command!(
        r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"left":true}}}}"##
    ) != 0
    {
        buttons |= 1;
    }
    if minecraft::command!(
        r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"right":true}}}}"##
    ) != 0
    {
        buttons |= 2;
    }
    if minecraft::command!(
        r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"forward":true}}}}"##
    ) != 0
    {
        buttons |= 4;
    }
    if minecraft::command!(
        r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"backward":true}}}}"##
    ) != 0
    {
        buttons |= 8;
    }
    if minecraft::command!(
        r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"jump":true}}}}"##
    ) != 0
    {
        buttons |= 16;
    }
    if minecraft::command!(
        r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"sneak":true}}}}"##
    ) != 0
    {
        buttons |= 32;
    }
    demo::celeste_set_input(buttons);
    let result = demo::step();
    demo::state::update(|s| s.frames += 1);
    result
}
