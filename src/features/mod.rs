use rand::Rng;

use crate::blocks::BlockID;

pub mod tree;
pub mod bush;

pub trait Feature {
    fn place(&self, x: i32, y: i32, z: i32, rand: &mut dyn Rng, set_block: impl FnMut(i32, i32, i32, BlockID));
}