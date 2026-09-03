use phf::phf_map;

use crate::blocks::{self, Block};

pub type ItemId = &'static str;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Item {
    pub id: ItemId,
    pub properties: ItemProperties,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ItemProperties {
    BlockItem(BlockItem),
    Nothing,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BlockItem {
    pub block: Block,
}

pub const NOTHING: Item = Item {
    id: "nothing",
    properties: ItemProperties::Nothing,
};

pub const GRASS: Item = Item {
    id: "grass",
    properties: ItemProperties::BlockItem(BlockItem { block: blocks::GRASS })
};

pub const ITEMS: phf::Map<ItemId, Item> = phf_map! {
    "nothing" => NOTHING,

};