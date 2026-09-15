use doom_demo as demo;
pub fn main()->i32 {
 if !demo::state().prepared {
  if minecraft::command!("execute unless entity @a")!=0{return 0;}
  return demo::lifecycle::prepare::main();
 }
 if !demo::state().ready {return 0;}
 let present=minecraft::command!("execute if entity @a")!=0;
 if present && !demo::state().present {minecraft::command!("tag @a remove doom_player");}
 demo::update(|s|s.present=present);
 if minecraft::command!("execute if entity @a[tag=!doom_player]")!=0 {demo::lifecycle::join::main();}
 minecraft::commands! {"scoreboard players enable @a doom_reset";"scoreboard players enable @a doom_pause";"scoreboard players enable @a doom_weapon";}
 if minecraft::command!("execute if entity @a[scores={doom_reset=1..}]")!=0 {demo::reset();}
 if minecraft::command!("execute if entity @a[scores={doom_pause=1..}]")!=0 {demo::update(|s|s.running=!s.running);}
 let weapon=minecraft::command!("scoreboard players get @a[limit=1] doom_weapon");demo::weapon(weapon);
 minecraft::commands! {"scoreboard players set @a[scores={doom_reset=1..}] doom_reset 0";"scoreboard players set @a[scores={doom_pause=1..}] doom_pause 0";"scoreboard players set @a[scores={doom_weapon=1..}] doom_weapon 0";}
 if present && demo::state().running {return demo::lifecycle::frame::main();}0
}
