use rand::Rng;

pub mod tree;
pub mod bush;
pub mod caves;
pub mod ore;
pub trait Feature {
    fn place(&self, x: i32, y: i32, z: i32, rand: &mut dyn Rng, set_block: impl FnMut(i32, i32, i32, crate::blocks::BlockID, fn(crate::blocks::BlockID) -> bool));
    fn feature_type(&self) -> FeatureType;
}

pub enum FeatureType {
    Surface,
    Carver,
    Ore,
}