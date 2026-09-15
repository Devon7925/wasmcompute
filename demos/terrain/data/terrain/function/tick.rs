pub fn main()->i32 {
    minecraft::commands! {
        "scoreboard players enable @a terrain_normal";
        "scoreboard players enable @a terrain_alternate";
        "scoreboard players enable @a terrain_seed";
        "scoreboard players enable @a terrain_generate";
    }
    if minecraft::command!("execute if entity @a[scores={terrain_normal=1..}]")!=0 {
        minecraft::command!("scoreboard players set @a terrain_normal 0");terrain_demo::begin(42,0,0);
    }
    if minecraft::command!("execute if entity @a[scores={terrain_alternate=1..}]")!=0 {
        minecraft::command!("scoreboard players set @a terrain_alternate 0");terrain_demo::begin(1337,0,0);
    }
    if minecraft::command!("execute if entity @a[scores={terrain_generate=1..}]")!=0 {
        let seed=minecraft::command!("scoreboard players get @a[scores={terrain_generate=1..},limit=1] terrain_seed");
        minecraft::command!("scoreboard players set @a terrain_generate 0");terrain_demo::begin(seed as i64,0,0);
    }
    if terrain_demo::busy() && !terrain_demo::instant(){
        let stage=terrain_demo::step();
        if stage<0{minecraft::command!("tellraw @a {text:'Terrain replacement complete. /trigger terrain_normal restores seed 42.',color:'green'}");}
        else{minecraft::command!("title @a actionbar {text:'Generating terrain: $(stage)/$(total)'}",stage=stage,total=terrain_demo::STAGES as i32);}
    }0
}
