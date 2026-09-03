use std::{collections::HashMap, hash::Hash, sync::{LazyLock, Mutex}};

use bespoke_engine::resource_compiler::AtlasSection;
use phf::phf_map;

use crate::items::ItemID;

pub type BlockID = u16;

pub const AIR: BlockID = 0;

pub const GRASS: BlockID = 1;

pub const WATER: BlockID = 2;

pub const STONE: BlockID = 3;

pub const DIRT: BlockID = 4;

pub const ROCK: BlockID = 5;

pub const LEAVES: BlockID = 6;

pub const GOLD: BlockID = 7;

pub const WOOD: BlockID = 8;

pub const PLANKS: BlockID = 9;

// pub const BLOCKS: phf::Map<BlockID, Block> = phf_map! {
//     0u16 => AIR,
//     1 => GRASS,
//     2 => WATER,
//     3 => STONE,
//     4 => DIRT,
//     5 => ROCK,
//     6 => LEAVES,
//     7 => GOLD,
//     8 => WOOD,
// };

// pub static CUSTOM_BLOCKS: LazyLock<Mutex<HashMap<BlockID, Block>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// pub fn get_block(id: BlockID) -> Block {
//     BLOCKS.get(&id).copied().unwrap_or_else(|| CUSTOM_BLOCKS.lock().unwrap().get(&id).unwrap().to_owned())
// }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub id: BlockID,
    pub solid: bool,
    pub color: [f32; 4],
    pub atlas_section: AtlasSection,
    pub has_model: bool,
    pub layer: i32,
    pub cull: bool,
    pub item: Option<ItemID>,
}

impl Hash for Block {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.solid.hash(state);
        self.atlas_section.hash(state);
        self.has_model.hash(state);
        self.layer.hash(state);
    }
}

impl Eq for Block {}

// pub fn solid_block(block: BlockID) -> bool {
//     get_block(block).solid
// }

pub const SOLID_LAYER: i32 = 0;
pub const TRANSPARENT_LAYER: i32 = 1;
pub const NOT_RENDERED_LAYER: i32 = 2;