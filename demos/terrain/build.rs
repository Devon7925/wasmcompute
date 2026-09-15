use std::{collections::BTreeSet,env,fs,fmt::Write,path::Path};
fn collect(v:&serde_json::Value,blocks:&mut BTreeSet<String>){
    match v {
        serde_json::Value::Object(map)=>{if let Some(name)=map.get("Name").and_then(|v|v.as_str()){blocks.insert(name.to_owned());}for v in map.values(){collect(v,blocks);}}
        serde_json::Value::Array(values)=>for v in values{collect(v,blocks);},_=>{}
    }
}
fn main(){
    println!("cargo:rerun-if-changed=kernel/assets");
    let settings:serde_json::Value=serde_json::from_str(&fs::read_to_string("kernel/assets/worldgen/noise_settings/overworld.json").unwrap()).unwrap();
    let registry:serde_json::Value=serde_json::from_str(&fs::read_to_string("kernel/assets/blocks.json").unwrap()).unwrap();
    let mut blocks=BTreeSet::new();collect(&settings,&mut blocks);
    for name in ["air","water","lava","copper_ore","raw_copper_block","granite","deepslate_iron_ore","raw_iron_block","tuff","orange_terracotta","yellow_terracotta","brown_terracotta","red_terracotta","white_terracotta","light_gray_terracotta","terracotta","packed_ice","snow_block"]{blocks.insert(format!("minecraft:{name}"));}
    let mut source=String::from("pub fn fill(block:u16,x:i32,z:i32,lo:i32,hi:i32)->i32 {match block {\n");
    for name in blocks {
        let id=registry[&name]["states"].as_array().unwrap().iter().find(|s|s["default"].as_bool()==Some(true)).unwrap()["id"].as_u64().unwrap();
        writeln!(source,"{id} => minecraft::command!(\"fill $(x) $(lo) $(z) $(x) $(hi) $(z) {name} strict\",x=x,lo=lo,z=z,hi=hi),").unwrap();
    }
    source.push_str("_=>panic!(\"unexpected generated block state\")}}\n");
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("blocks.rs"),source).unwrap();
}
