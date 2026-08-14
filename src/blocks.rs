use std::{collections::HashMap, hash::Hash, sync::{LazyLock, Mutex}};

use phf::phf_map;

pub type BlockID = u16;

pub const AIR: Block = Block {
    id: 0,
    solid: false,
    color: [0.0; 4],
    atlas_x: 0,
    atlas_y: 0,
    has_model: false,
    layer: NOT_RENDERED_LAYER,
    cull: true,
};
pub const GRASS: Block = Block {
    id: 1,
    solid: true,
    color: [0.0, 1.0, 0.0, 1.0],
    atlas_x: 0,
    atlas_y: 0,
    has_model: false,
    layer: SOLID_LAYER,
    cull: true,
};
pub const WATER: Block = Block {
    id: 2,
    solid: false,
    color: [0.0, 0.0, 1.0, 0.5],
    atlas_x: 0,
    atlas_y: 15,
    has_model: false,
    layer: TRANSPARENT_LAYER,
    cull: true,
};

pub const STONE: Block = Block {
    id: 3,
    solid: true,
    color: [0.0, 0.0, 0.0, 1.0],
    atlas_x: 3,
    atlas_y: 0,
    has_model: false,
    layer: SOLID_LAYER,
    cull: true,
};

pub const DIRT: Block = Block {
    id: 4,
    solid: true,
    color: [0.0, 0.0, 0.0, 1.0],
    atlas_x: 2,
    atlas_y: 0,
    has_model: false,
    layer: SOLID_LAYER,
    cull: true,
};

pub const GOLD: Block = Block {
    id: 7,
    solid: true,
    color: [0.0, 0.0, 0.0, 1.0],
    atlas_x: 6,
    atlas_y: 2,
    has_model: false,
    layer: SOLID_LAYER,
    cull: true,
};

pub const ROCK: Block = Block {
    id: 5,
    solid: false,
    color: [0.0, 0.0, 0.0, 1.0],
    atlas_x: 3,
    atlas_y: 0,
    has_model: true,
    layer: SOLID_LAYER,
    cull: true,
};

pub const LEAVES: Block = Block {
    id: 6,
    solid: true,
    color: [0.0, 0.0, 0.0, 1.0],
    atlas_x: 4,
    atlas_y: 1,
    has_model: false,
    layer: SOLID_LAYER,
    cull: false,
};

pub const BLOCKS: phf::Map<BlockID, Block> = phf_map! {
    0u16 => AIR,
    1 => GRASS,
    2 => WATER,
    3 => STONE,
    4 => DIRT,
    5 => ROCK,
    6 => LEAVES,
    7 => GOLD,
};

pub const ATLAS_X_BLOCKS: u32 = 16;
pub const ATLAS_Y_BLOCKS: u32 = 16;

pub static CUSTOM_BLOCKS: LazyLock<Mutex<HashMap<BlockID, Block>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn get_block(id: BlockID) -> Block {
    BLOCKS.get(&id).copied().unwrap_or_else(|| CUSTOM_BLOCKS.lock().unwrap().get(&id).unwrap().to_owned())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub id: BlockID,
    pub solid: bool,
    pub color: [f32; 4],
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub has_model: bool,
    pub layer: i32,
    pub cull: bool,
}

impl Hash for Block {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.solid.hash(state);
        self.atlas_x.hash(state);
        self.atlas_y.hash(state);
        self.has_model.hash(state);
        self.layer.hash(state);
    }
}

impl Eq for Block {}

pub fn solid_block(block: BlockID) -> bool {
    get_block(block).solid
}

pub const SOLID_LAYER: i32 = 0;
pub const TRANSPARENT_LAYER: i32 = 1;
pub const NOT_RENDERED_LAYER: i32 = 2;