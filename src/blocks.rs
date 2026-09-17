use std::{hash::Hash, time::Duration};

use bespoke_engine::{resource_compiler::AtlasSection};

use crate::{BLOCK_ATLAS_PNG_COBBLESTONE_SECTION, BLOCK_ATLAS_PNG_COPPER_ORE_SECTION, BLOCK_ATLAS_PNG_DIRT_SECTION, BLOCK_ATLAS_PNG_GRASS_SECTION, BLOCK_ATLAS_PNG_IRON_ORE_SECTION, BLOCK_ATLAS_PNG_KILN_FRONT_SECTION, BLOCK_ATLAS_PNG_LEAVES_SECTION, BLOCK_ATLAS_PNG_PLANKS_SECTION, BLOCK_ATLAS_PNG_RAINBOW_SECTION, BLOCK_ATLAS_PNG_STONE_SECTION, BLOCK_ATLAS_PNG_WATER_SECTION, BLOCK_ATLAS_PNG_WOOD_SECTION, GENERATED_KILN_BLOCK_MODEL, GENERATED_ROCK_BLOCK_MODEL, const_block_model_types::BlockModelTrait, items::{self, ItemID}, registries::BlockRegistry};

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

pub const COBBLESTONE: BlockID = 10;

pub const IRON_ORE: BlockID = 11;

pub const COPPER_ORE: BlockID = 12;

pub const KILN: BlockID = 13;

impl BlockRegistry {
    pub fn register_all(&mut self) {
        self.register(Block {
            id: AIR,
            solid: false,
            color: [0.0; 4],
            atlas_section: BLOCK_ATLAS_PNG_DIRT_SECTION,
            model: None,
            layer: NOT_RENDERED_LAYER,
            cull: true,
            item: None,
            break_duration: Duration::ZERO,
            drops: items::NOTHING,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: GRASS,
            solid: true,
            color: [0.0, 1.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_GRASS_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::GRASS_BLOCK),
            break_duration: Duration::from_secs_f32(0.7),
            drops: items::GRASS_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: WATER,
            solid: false,
            color: [0.0, 0.0, 1.0, 0.8],
            atlas_section: BLOCK_ATLAS_PNG_WATER_SECTION,
            model: None,
            layer: TRANSPARENT_LAYER,
            cull: true,
            item: None,
            break_duration: Duration::ZERO,
            drops: items::NOTHING,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: STONE,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_STONE_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::STONE_BLOCK),
            break_duration: Duration::from_secs_f32(6.0),
            drops: items::COBBLESTONE_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: DIRT,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_DIRT_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::DIRT_BLOCK),
            break_duration: Duration::from_secs_f32(0.6),
            drops: items::DIRT_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: ROCK,
            solid: false,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_STONE_SECTION,
            model: Some(GENERATED_ROCK_BLOCK_MODEL),
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::ROCK_BLOCK),
            break_duration: Duration::from_secs_f32(0.3),
            drops: items::ROCK_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: LEAVES,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_LEAVES_SECTION,
            model: None,
            layer: SEMITRANSPARENT_LAYER,
            cull: false,
            item: Some(items::LEAVES_BLOCK),
            break_duration: Duration::from_secs_f32(0.3),
            drops: items::LEAVES_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: GOLD,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_RAINBOW_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::GOLD_BLOCK),
            break_duration: Duration::from_secs_f32(0.1),
            drops: items::GOLD_BLOCK,
            lighting_emission: [5; 3],
        });
        self.register(Block {
            id: WOOD,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_WOOD_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::WOOD_BLOCK),
            break_duration: Duration::from_secs_f32(3.0),
            drops: items::WOOD_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: PLANKS,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_PLANKS_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::PLANKS_BLOCK),
            break_duration: Duration::from_secs_f32(2.0),
            drops: items::PLANKS_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: COBBLESTONE,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_COBBLESTONE_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::COBBLESTONE_BLOCK),
            break_duration: Duration::from_secs_f32(6.0),
            drops: items::COBBLESTONE_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: IRON_ORE,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_IRON_ORE_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::IRON_ORE_BLOCK),
            break_duration: Duration::from_secs_f32(8.0),
            drops: items::IRON_ORE_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: COPPER_ORE,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_COPPER_ORE_SECTION,
            model: None,
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::COPPER_ORE_BLOCK),
            break_duration: Duration::from_secs_f32(7.0),
            drops: items::COPPER_ORE_BLOCK,
            lighting_emission: [0; 3],
        });
        self.register(Block {
            id: KILN,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_KILN_FRONT_SECTION,
            model: Some(GENERATED_KILN_BLOCK_MODEL),
            layer: SOLID_LAYER,
            cull: true,
            item: Some(items::KILN_BLOCK),
            break_duration: Duration::from_secs_f32(7.0),
            drops: items::KILN_BLOCK,
            lighting_emission: [0; 3],
        });
    }
}

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

#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub id: BlockID,
    pub solid: bool,
    pub color: [f32; 4],
    pub atlas_section: AtlasSection,
    pub model: Option<&'static dyn BlockModelTrait>,
    pub layer: usize,
    pub cull: bool,
    pub item: Option<ItemID>,
    pub drops: ItemID,
    pub break_duration: Duration,
    pub lighting_emission: [u8; 3],
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }

    fn ne(&self, other: &Self) -> bool {
        self.id.ne(&other.id)
    }
}

impl Hash for Block {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Block {}

// pub fn solid_block(block: BlockID) -> bool {
//     get_block(block).solid
// }

pub const SOLID_LAYER: usize = 0;
pub const SEMITRANSPARENT_LAYER: usize = 1;
pub const TRANSPARENT_LAYER: usize = 2;
pub const NOT_RENDERED_LAYER: usize = 3;