use std::{fs,fmt::Write};
pub fn build() {
    let blocks:serde_json::Value=serde_json::from_str(&fs::read_to_string("assets/blocks.json").expect("vanilla block report")).expect("block report JSON");
    let mut out=String::from("#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash,Default)] pub struct BlockStateId(pub u16);\npub struct Block {pub id:u16,pub name:&'static str}\nimpl Block {pub const fn default_state(&self)->BlockStateId{BlockStateId(self.id)}}\npub struct Blocks; impl Blocks {pub fn get_default_state_id(&self,b:&Block)->BlockStateId{b.default_state()}}\npub struct Registry{pub blocks:Blocks,pub biomes:Biomes} pub static REGISTRY:Registry=Registry{blocks:Blocks,biomes:Biomes};\npub trait RegistryExt {}\npub mod blocks {pub mod block_state_ext {pub trait BlockStateExt {}}}\npub mod biome {pub type BiomeRef = &'static super::Biome; #[derive(Clone,Copy)] pub enum TemperatureModifier{None,Frozen}} pub struct Biome{pub id:usize,pub temperature:f32,pub temperature_modifier:biome::TemperatureModifier} pub struct Biomes; impl Biomes{pub fn by_id(&self,id:usize)->Option<&'static Biome>{BIOMES.get(id).copied()}} pub trait RegistryEntry{fn id(&self)->usize;} impl RegistryEntry for Biome{fn id(&self)->usize{self.id}}\npub mod vanilla_blocks {use super::Block;\n");
    for (name,b) in blocks.as_object().expect("blocks object") {
        let id=b["states"].as_array().expect("states").iter().find(|s|s["default"].as_bool()==Some(true)).expect("default state")["id"].as_u64().expect("state id");
        writeln!(out,"pub static {}:Block=Block{{id:{id},name:{name:?}}};",name.trim_start_matches("minecraft:").to_uppercase()).unwrap();
    }
    out.push_str("}\npub mod vanilla_biomes {use super::Biome;\n");
    // Dense local tags for equality in surface rules, never network registry IDs.
    let mut names:Vec<_>=fs::read_dir("assets/worldgen/biome").unwrap().map(|e|e.unwrap().path().file_stem().unwrap().to_str().unwrap().to_owned()).collect();
    names.sort();
    for (id,name) in names.iter().enumerate() {
        let b:serde_json::Value=serde_json::from_str(&fs::read_to_string(format!("assets/worldgen/biome/{name}.json")).unwrap()).unwrap();
        let temperature=b["temperature"].as_f64().expect("biome temperature");
        let modifier=if b["temperature_modifier"].as_str()==Some("frozen"){"Frozen"}else{"None"};
        writeln!(out,"pub static {}:&Biome=&Biome{{id:{id},temperature:{temperature:?}f32,temperature_modifier:super::biome::TemperatureModifier::{modifier}}};",name.trim_start_matches("minecraft:").to_uppercase()).unwrap();
    }
    out.push_str("}\npub static BIOMES:&[&Biome]=&[");
    for name in &names {writeln!(out,"vanilla_biomes::{},",name.to_uppercase()).unwrap();}
    out.push_str("];\nimpl BlockStateId {pub fn is_air(self)->bool{self==vanilla_blocks::AIR.default_state()||self==vanilla_blocks::CAVE_AIR.default_state()||self==vanilla_blocks::VOID_AIR.default_state()}}\n");fs::create_dir_all("src/generated").unwrap();fs::write("src/generated/registry.rs",out).unwrap();
}
