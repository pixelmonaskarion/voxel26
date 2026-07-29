use cgmath::{Vector3, vec3};

use crate::{blocks::solid_block, chunk::ChunkManager};

pub struct Player {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub time_since_ground: f64,
    pub movement_move: i32,
}

impl Player {
    pub fn new(position: Vector3<f32>) -> Self {
        Self {
            position,
            velocity: vec3(0.0, 0.0, 0.0),
            time_since_ground: 1.0,
            movement_move: 0,
        }
    }

    pub fn move_player(&mut self, delta: Vector3<f32>, world: &ChunkManager) {
        let x_steps = (delta.x.abs()/0.5).ceil();
        for _ in 0..x_steps as i32 {
            self.position.x += delta.x/x_steps;
            if self.colliding_world(world) {
                self.position.x -= delta.x/x_steps;
                self.velocity.x = 0.0;
            }
        }
        let z_steps = (delta.z.abs()/0.5).ceil();
        for _ in 0..z_steps as i32 {
            self.position.z += delta.z/z_steps;
            if self.colliding_world(world) {
                self.position.z -= delta.z/z_steps;
                self.velocity.z = 0.0;
            }
        }
        let y_steps = (delta.y.abs()/0.5).ceil();
        for _ in 0..y_steps as i32 {
            self.position.y += delta.y/y_steps;
            if self.colliding_world(world) {
                self.position.y -= delta.y/y_steps;
                self.velocity.y = 0.0;
                if delta.y < 0.0 {
                    self.time_since_ground = 0.0;
                }
            }
        }
    }

    pub fn colliding_world(&self, world: &ChunkManager) -> bool {
        if self.movement_move == 1 {
            return false;
        }
        let point_offsets = [
            vec3(-0.5, 0.5, -0.5),
            vec3(0.5, 0.5, -0.5),
            vec3(0.5, 0.5, 0.5),
            vec3(-0.5, 0.5, 0.5),

            vec3(-0.5, -0.5, -0.5),
            vec3(0.5, -0.5, -0.5),
            vec3(0.5, -0.5, 0.5),
            vec3(-0.5, -0.5, 0.5),

            vec3(-0.5, -1.5, -0.5),
            vec3(0.5, -1.5, -0.5),
            vec3(0.5, -1.5, 0.5),
            vec3(-0.5, -1.5, 0.5),
        ];
        for offset in point_offsets {
            let position = self.position+offset;
            let block_position = position.map(|it| it.floor() as i32).into();
            let block = world.get_block(block_position);
            if solid_block(block) {
                return true;
            }
        }
        return false;
    }
}