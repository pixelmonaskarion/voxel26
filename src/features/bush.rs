use rand::{Rng, RngExt};

use crate::{blocks::LEAVES, features::Feature};

pub struct BushFeature {

}

impl Feature for BushFeature {
    fn place(&self, mut x: i32, mut y: i32, mut z: i32, rand: &mut dyn Rng, mut set_block: impl FnMut(i32, i32, i32, crate::blocks::BlockID)) {
        while rand.random_range(0.0..1.0) < 0.8 {
            set_block(x, y, z, LEAVES.id);
            x += rand.random_range(-1..2);
            y += rand.random_range(-1..2);
            z += rand.random_range(-1..2);
        }
    }
}