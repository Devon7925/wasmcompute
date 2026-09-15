pub fn main()->i32 {
 let mut buttons=0u32;
 if minecraft::command!(r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"left":true}}}}"##)!=0 {buttons|=1<<0;}
 if minecraft::command!(r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"right":true}}}}"##)!=0 {buttons|=1<<1;}
 if minecraft::command!(r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"forward":true}}}}"##)!=0 {buttons|=1<<2;}
 if minecraft::command!(r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"backward":true}}}}"##)!=0 {buttons|=1<<3;}
 if minecraft::command!(r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"jump":true}}}}"##)!=0 {buttons|=1<<4;}
 if minecraft::command!(r##"execute as @a[limit=1] if predicate {"type":"minecraft:entity_properties","entity":"this","predicate":{"minecraft:type_specific/player":{"input":{"sneak":true}}}}"##)!=0 {buttons|=1<<5;}
doom_demo::doom_set_input(buttons);doom_demo::step()
}
