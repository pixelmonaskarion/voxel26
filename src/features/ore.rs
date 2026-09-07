use std::ops::Range;

use cgmath::vec3;
use rand::{Rng, RngExt};
use rustc_hash::FxHashSet;

use crate::{blocks::{self, BlockID}, features::Feature};

pub struct OreFeature {
    pub num_blocks: Range<usize>,
    pub block: BlockID,
}

impl Feature for OreFeature {
    fn place(&self, mut x: i32, mut y: i32, mut z: i32, rand: &mut dyn Rng, mut set_block: impl FnMut(i32, i32, i32, crate::blocks::BlockID, fn(crate::blocks::BlockID) -> bool)) {
        let mut num_blocks = rand.random_range(self.num_blocks.clone());
        let mut already_placed = FxHashSet::default();
        while num_blocks > 0 {
            let direction = [vec3(1, 0, 0), vec3(-1, 0, 0), vec3(0, -1, 0), vec3(0, 1, 0), vec3(0, 0, -1), vec3(0, 0, -1)][rand.random_range(0..6)];
            x += direction.x;
            y += direction.y;
            z += direction.z;
            if !already_placed.contains(&vec3(x, y, z)) {
                set_block(x, y, z, self.block, |block| block == blocks::STONE );
                already_placed.insert(vec3(x, y, z));
                num_blocks -= 1;
            }
        }
    }

    fn feature_type(&self) -> super::FeatureType {
        super::FeatureType::Ore
    }
}