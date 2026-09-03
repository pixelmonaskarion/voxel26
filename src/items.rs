use crate::blocks::BlockID;

pub type ItemID = &'static str;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Item {
    pub id: ItemID,
    pub properties: ItemProperties,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemProperties {
    BlockItem(BlockItem),
    Nothing,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlockItem {
    pub block: BlockID,
}

pub const NOTHING: ItemID = "nothing";