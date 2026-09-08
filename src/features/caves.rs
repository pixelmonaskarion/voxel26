use glam::{dvec3, ivec3};
use noise::{NoiseFn, Perlin};
use rand::{Rng, RngExt};

use crate::{blocks, features::Feature};

pub struct WormCavesFeature {

}

impl Feature for WormCavesFeature {
    fn place(&self, x: i32, y: i32, z: i32, rand: &mut dyn Rng, mut set_block: impl FnMut(i32, i32, i32, crate::blocks::BlockID, fn(crate::blocks::BlockID) -> bool)) {
        set_block(x, y, z, blocks::GOLD, |_| true);
        let length = rand.random_range(25..100);
        let x_perlin = Perlin::new(rand.random());
        let y_perlin = Perlin::new(rand.random());
        let z_perlin = Perlin::new(rand.random());
        let size_perlin = Perlin::new(rand.random());
        let mut worm_pos = ivec3(x, y, z);
        let size_scale = 1.0/20.0;
        let direction_scale = 1.0/20.0;
        for _ in 0..length {
            let pos = worm_pos.as_dvec3()*direction_scale;
            let dx = x_perlin.get(pos.into());
            let dy = y_perlin.get(pos.into());
            let dz = z_perlin.get(pos.into());
            let pos = worm_pos.as_dvec3()*size_scale;
            
            fn lerp(a: f64, b: f64, t: f64) -> f64 {
                a + (b - a) * t
            }

            let size = lerp(1.5, 5.0, (size_perlin.get(pos.into())+1.0)/2.0);
            let size2 = size.powi(2);
            let direction = dvec3(dx, dy, dz).normalize()*3.0;
            worm_pos += direction.round().as_ivec3();
            for offset_x in -size.round() as i32..=size.round() as i32 {
                for offset_y in -size.round() as i32..=size.round() as i32 {
                    for offset_z in -size.round() as i32..=size.round() as i32 {
                        let offset = ivec3(offset_x, offset_y, offset_z);
                        if offset.distance_squared(ivec3(0, 0, 0)) < size2.round() as i32 {
                            let actual_pos = worm_pos+offset;
                            set_block(actual_pos.x, actual_pos.y, actual_pos.z, blocks::AIR, |block| block != blocks::WATER );
                        }
                    }
                }
            }
        }
    }

    fn feature_type(&self) -> super::FeatureType {
        super::FeatureType::Carver
    }
}