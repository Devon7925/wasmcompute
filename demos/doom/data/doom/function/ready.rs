pub fn main()->i32 {
 let mut loaded=true;for x in (0..128).step_by(16){loaded &= minecraft::command!("execute if loaded $(x) 80 64",x=x)!=0;}
 loaded &= minecraft::command!("execute if loaded 54 119 152 if loaded 74 119 170")!=0;
 if !loaded{minecraft::command!("schedule function doom:ready 2t replace");return 0;}
 minecraft::commands! {"fill 54 119 152 74 119 170 minecraft:black_concrete strict";"fill 0 80 63 127 159 63 minecraft:black_concrete strict";}
 let draws=doom_demo::reset();doom_demo::update(|s|s.ready=true);draws
}
