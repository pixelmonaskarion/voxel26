use glam::Vec3;

use crate::{chunk::{CHUNK_SIZE, ChunkManager}, registries::Registries};

pub struct RingIter {
    max: i32,
    d: i32,
    x: i32,
    y: i32,
    z_sign: bool,
    finished: bool,
}

pub fn positions(max_range: i32) -> RingIter {
    RingIter {
        max: max_range,
        d: 0,
        x: 0,
        y: 0,
        z_sign: false,
        finished: false,
    }
}

impl Iterator for RingIter {
    type Item = [i32; 3];

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            let rem1 = self.d - self.x.abs();
            let rem2 = rem1 - self.y.abs();

            let z = if self.z_sign { -rem2 } else { rem2 };
            let out = [self.x, self.y, z];

            if rem2 > 0 && !self.z_sign {
                self.z_sign = true;
                return Some(out);
            }

            self.z_sign = false;

            self.y += 1;
            if self.y > rem1 {
                self.x += 1;
                if self.x > self.d {
                    self.d += 1;
                    if self.d > self.max {
                        self.finished = true;
                        return None;
                    }
                    self.x = -self.d;
                }
                let rem1 = self.d - self.x.abs();
                self.y = -rem1;
            }

            return Some(out);
        }
    }
}

pub fn neighbors(chunk_pos: [i32; 3]) -> [[i32; 3]; 6] {
    [
        [chunk_pos[0]+1, chunk_pos[1], chunk_pos[2]],
        [chunk_pos[0]-1, chunk_pos[1], chunk_pos[2]],
        [chunk_pos[0], chunk_pos[1]+1, chunk_pos[2]],
        [chunk_pos[0], chunk_pos[1]-1, chunk_pos[2]],
        [chunk_pos[0], chunk_pos[1], chunk_pos[2]+1],
        [chunk_pos[0], chunk_pos[1], chunk_pos[2]-1],
    ]
}

pub fn chunk_for_block_position(block_position: [i32; 3]) -> [i32; 3] {
    let x = block_position[0];
    let y = block_position[1];
    let z = block_position[2];
    let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
    let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
    let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
    return [cx, cy, cz];
}

pub fn chunk_for_world_position(world_position: [f32; 3]) -> [i32; 3] {
    let x = world_position[0];
    let y = world_position[1];
    let z = world_position[2];
    let cx = (x/CHUNK_SIZE as f32).floor() as i32;
    let cy = (y/CHUNK_SIZE as f32).floor() as i32;
    let cz = (z/CHUNK_SIZE as f32).floor() as i32;
    return [cx, cy, cz];
}

pub fn colliding_world(world: &ChunkManager, center: Vec3, positive_size: Vec3, negative_size: Vec3, registries: &Registries) -> bool {
    let min_world = center + negative_size;
    let max_world = center + positive_size;

    let min_block_x = min_world.x.floor() as i32;
    let max_block_x = max_world.x.floor() as i32;
    let min_block_y = min_world.y.floor() as i32;
    let max_block_y = max_world.y.floor() as i32;
    let min_block_z = min_world.z.floor() as i32;
    let max_block_z = max_world.z.floor() as i32;

    for bx in min_block_x..=max_block_x {
        for by in min_block_y..=max_block_y {
            for bz in min_block_z..=max_block_z {
                let block_position = [bx, by, bz];
                let block = world.get_block(block_position);
                if registries.block_registry.get_block(&block).solid {
                    return true;
                }
            }
        }
    }
    return false;
}