use rand::{Rng, RngExt};

use crate::{blocks::{self, LEAVES, WOOD}, features::Feature};

pub struct TreeFeature {

}

impl Feature for TreeFeature {
    fn place(&self, x: i32, y: i32, z: i32, rand: &mut dyn Rng, mut set_block: impl FnMut(i32, i32, i32, crate::blocks::BlockID, fn(crate::blocks::BlockID) -> bool)) {
        let length = rand.random_range(3..6);
        for l in 0..length {
            set_block(x, y+l, z, WOOD, |block| { block == blocks::AIR || block == blocks::LEAVES });
        }
        for lx in x-2..x+3 {
            for lz in z-2..z+3 {
                for ly in y+length-2..y+length {
                    set_block(lx, ly, lz, LEAVES, |block| { block == blocks::AIR });
                }
            }
        }
        for ly in y+length..y+length+2 {
            for (lx, lz) in [(1, 0), (0, 1), (0, 0), (-1, 0), (0, -1)] {
                set_block(x+lx, ly, z+lz, LEAVES, |block| block == blocks::AIR );
            }
        }
    }

    fn feature_type(&self) -> super::FeatureType {
        super::FeatureType::Surface
    }
}