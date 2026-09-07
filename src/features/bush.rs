use rand::{Rng, RngExt};

use crate::{blocks, features::Feature};

pub struct BushFeature {

}

impl Feature for BushFeature {
    fn place(&self, mut x: i32, mut y: i32, mut z: i32, rand: &mut dyn Rng, mut set_block: impl FnMut(i32, i32, i32, crate::blocks::BlockID, fn(crate::blocks::BlockID) -> bool)) {
        while rand.random_range(0.0..1.0) < 0.8 {
            set_block(x, y, z, blocks::LEAVES, |block| block == blocks::AIR );
            x += rand.random_range(-1..2);
            y += rand.random_range(-1..2);
            z += rand.random_range(-1..2);
        }
    }

    fn feature_type(&self) -> super::FeatureType {
        super::FeatureType::Surface
    }
}