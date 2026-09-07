use crate::{RES_TAGS_BLOCKS_AXE_BREAKABLE_JSON, RES_TAGS_BLOCKS_PICKAXE_BREAKABLE_JSON, RES_TAGS_BLOCKS_SHOVEL_BREAKABLE_JSON, RES_TAGS_ITEMS_HAMMERS_JSON, registries::{ItemRegistry, TagID, TagRegistry}};

pub const PICKAXE_BREAKABLE_TAG: TagID = "pickaxe_breakable";
pub const AXE_BREAKABLE_TAG: TagID = "axe_breakable";
pub const SHOVEL_BREAKABLE_TAG: TagID = "shovel_breakable";
pub const HAMMERS_TAG: TagID = "hammers";

impl TagRegistry {
    pub fn register_all(&mut self, item_registry: &ItemRegistry) {
        self.register_block_tag(PICKAXE_BREAKABLE_TAG, RES_TAGS_BLOCKS_PICKAXE_BREAKABLE_JSON.load_string());
        self.register_block_tag(AXE_BREAKABLE_TAG, RES_TAGS_BLOCKS_AXE_BREAKABLE_JSON.load_string());
        self.register_block_tag(SHOVEL_BREAKABLE_TAG, RES_TAGS_BLOCKS_SHOVEL_BREAKABLE_JSON.load_string());
        self.register_item_tag(HAMMERS_TAG, RES_TAGS_ITEMS_HAMMERS_JSON.load_string(), item_registry);
    }
}