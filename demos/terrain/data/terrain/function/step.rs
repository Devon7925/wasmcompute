pub fn main()->i32{
    let result=terrain_demo::step();
    if result<0{minecraft::command!("tellraw @a {text:'Terrain replacement complete.',color:'green'}");}
    result
}
