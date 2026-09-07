use std::collections::HashMap;

use bespoke_engine::resource_compiler::AtlasSection;

use crate::{ITEM_ATLAS_PNG_CRUSHED_COPPER_ORE_SECTION, ITEM_ATLAS_PNG_STICK_SECTION, ITEM_ATLAS_PNG_STONE_AXE_SECTION, ITEM_ATLAS_PNG_STONE_HAMMER_SECTION, ITEM_ATLAS_PNG_STONE_PICKAXE_SECTION, ITEM_ATLAS_PNG_STONE_SHOVEL_SECTION, ITEM_ATLAS_PNG_WOODEN_AXE_SECTION, ITEM_ATLAS_PNG_WOODEN_PICKAXE_SECTION, ITEM_ATLAS_PNG_WOODEN_SHOVEL_SECTION, blocks::BlockID, player::{AttributeModifier, AttributeModifierCondition, EntityAttribute}, registries::ItemRegistry, tags::{AXE_BREAKABLE_TAG, PICKAXE_BREAKABLE_TAG, SHOVEL_BREAKABLE_TAG}};

pub type ItemID = &'static str;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Item {
    pub id: ItemID,
    pub properties: ItemProperties,
    pub attribute_modifiers: HashMap<EntityAttribute, ConditionalAttributeModifier>
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
pub struct ConditionalAttributeModifier {
    pub modifier: AttributeModifier,
    pub condition: AttributeModifierCondition,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemProperties {
    BlockItem(BlockItem),
    BasicItem(BasicItem),
    Nothing,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlockItem {
    pub block: BlockID,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BasicItem {
    pub section: AtlasSection,
}

pub const NOTHING: ItemID = "nothing";
pub const STICK: ItemID = "stick";
pub const WOODEN_PICKAXE: ItemID = "wooden_pickaxe";
pub const WOODEN_AXE: ItemID = "wooden_axe";
pub const WOODEN_SHOVEL: ItemID = "wooden_shovel";
pub const STONE_PICKAXE: ItemID = "stone_pickaxe";
pub const STONE_AXE: ItemID = "stone_axe";
pub const STONE_SHOVEL: ItemID = "stone_shovel";
pub const STONE_HAMMER: ItemID = "stone_hammer";
pub const CRUSHED_COPPER_ORE: ItemID = "crushed_copper_ore";

pub const GRASS_BLOCK: ItemID = "grass_block";
pub const STONE_BLOCK: ItemID = "stone_block";
pub const COBBLESTONE_BLOCK: ItemID = "cobblestone_block";
pub const DIRT_BLOCK: ItemID = "dirt_block";
pub const ROCK_BLOCK: ItemID = "rock_block";
pub const LEAVES_BLOCK: ItemID = "leaves_block";
pub const GOLD_BLOCK: ItemID = "gold_block";
pub const WOOD_BLOCK: ItemID = "wood_block";
pub const PLANKS_BLOCK: ItemID = "planks_block";
pub const IRON_ORE_BLOCK: ItemID = "iron_ore_block";
pub const COPPER_ORE_BLOCK: ItemID = "copper_ore_block";
pub const KILN_BLOCK: ItemID = "kiln_block";

impl ItemRegistry {
    pub fn register_all(&mut self) {
        self.register(Item {
            id: NOTHING,
            properties: ItemProperties::Nothing,
            attribute_modifiers: Default::default(),
        });
        self.register(Item {
            id: STICK,
            properties: ItemProperties::BasicItem(BasicItem {
                section: ITEM_ATLAS_PNG_STICK_SECTION
            }),
            attribute_modifiers: Default::default(),
        });
        self.register(Item {
            id: WOODEN_PICKAXE,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_WOODEN_PICKAXE_SECTION,
            }),
            attribute_modifiers: HashMap::from_iter([(EntityAttribute::BlockBreakSpeed, ConditionalAttributeModifier { condition: AttributeModifierCondition::TargetInTag(PICKAXE_BREAKABLE_TAG), modifier: crate::player::AttributeModifier::Multiply(3.0.into()) })]),
        });
        self.register(Item {
            id: WOODEN_AXE,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_WOODEN_AXE_SECTION
            }),
            attribute_modifiers: HashMap::from_iter([(EntityAttribute::BlockBreakSpeed, ConditionalAttributeModifier { condition: AttributeModifierCondition::TargetInTag(AXE_BREAKABLE_TAG), modifier: crate::player::AttributeModifier::Multiply(3.0.into()) })]),
        });
        self.register(Item {
            id: WOODEN_SHOVEL,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_WOODEN_SHOVEL_SECTION
            }),
            attribute_modifiers: HashMap::from_iter([(EntityAttribute::BlockBreakSpeed, ConditionalAttributeModifier { condition: AttributeModifierCondition::TargetInTag(SHOVEL_BREAKABLE_TAG), modifier: crate::player::AttributeModifier::Multiply(3.0.into()) })]),
        });

        self.register(Item {
            id: STONE_PICKAXE,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_STONE_PICKAXE_SECTION
            }),
            attribute_modifiers: HashMap::from_iter([(EntityAttribute::BlockBreakSpeed, ConditionalAttributeModifier { condition: AttributeModifierCondition::TargetInTag(PICKAXE_BREAKABLE_TAG), modifier: crate::player::AttributeModifier::Multiply(6.0.into()) })]),
        });
        self.register(Item {
            id: STONE_AXE,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_STONE_AXE_SECTION
            }),
            attribute_modifiers: HashMap::from_iter([(EntityAttribute::BlockBreakSpeed, ConditionalAttributeModifier { condition: AttributeModifierCondition::TargetInTag(AXE_BREAKABLE_TAG), modifier: crate::player::AttributeModifier::Multiply(6.0.into()) })]),
        });
        self.register(Item {
            id: STONE_SHOVEL,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_STONE_SHOVEL_SECTION
            }),
            attribute_modifiers: HashMap::from_iter([(EntityAttribute::BlockBreakSpeed, ConditionalAttributeModifier { condition: AttributeModifierCondition::TargetInTag(SHOVEL_BREAKABLE_TAG), modifier: crate::player::AttributeModifier::Multiply(6.0.into()) })]),
        });
        self.register(Item {
            id: STONE_HAMMER,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_STONE_HAMMER_SECTION
            }),
            attribute_modifiers: Default::default(),
        });
        self.register(Item {
            id: CRUSHED_COPPER_ORE,
            properties: ItemProperties::BasicItem(BasicItem { 
                section: ITEM_ATLAS_PNG_CRUSHED_COPPER_ORE_SECTION,
            }),
            attribute_modifiers: Default::default(),
        });
    }
}