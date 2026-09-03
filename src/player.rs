use std::time::Duration;

use bespoke_engine::{binding::UniformBinding, surface_context::SurfaceCtx, texture::Texture};
use cgmath::{InnerSpace, Vector3, vec3};

use crate::{blocks::solid_block, chunk::ChunkManager, inventory::{Inventory, ItemAtlas}};

pub struct Player {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub time_since_ground: Duration,
    pub movement_mode: i32,
    pub break_cooldown: Duration,

    pub inventory: Inventory,
    pub health: f32,
}

impl Player {
    pub fn new(position: Vector3<f32>, item_atlas: &mut ItemAtlas, block_atlas: &UniformBinding<Texture>, surface_ctx: &dyn SurfaceCtx) -> Self {
        let mut _self = Self {
            position,
            velocity: vec3(0.0, 0.0, 0.0),
            time_since_ground: Duration::new(2, 0),
            movement_mode: 0,
            break_cooldown: Duration::ZERO,
            inventory: Inventory::empty_size(4*9, item_atlas, block_atlas, surface_ctx),
            health: 20.0,
        };
        _self
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
                    self.time_since_ground = Duration::ZERO;
                }
            }
        }
    }

    pub fn damage(&mut self, damage: f32) {
        self.health -= damage;
    }

    pub fn colliding_world(&self, world: &ChunkManager) -> bool {
        if self.movement_mode == 1 {
            return false;
        }
        let width = 0.8;
        let height = 1.8;
        let half_w = width / 2.0;
        let top = 0.3;
        let mid = top - height / 2.0;
        let bottom = top - height;
        let point_offsets = [
            vec3(-half_w, top, -half_w),
            vec3(half_w, top, -half_w),
            vec3(half_w, top, half_w),
            vec3(-half_w, top, half_w),

            vec3(-half_w, mid, -half_w),
            vec3(half_w, mid, -half_w),
            vec3(half_w, mid, half_w),
            vec3(-half_w, mid, half_w),

            vec3(-half_w, bottom, -half_w),
            vec3(half_w, bottom, -half_w),
            vec3(half_w, bottom, half_w),
            vec3(-half_w, bottom, half_w),
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

    pub fn raycast(&self, direction: Vector3<f32>, max_distance: f32, world: &ChunkManager) -> Option<([i32;3], BlockFace)> {
        if direction.magnitude2() == 0.0 {
            return None;
        }
        let dir = direction.normalize();
        let mut voxel: [i32; 3] = self.position.map(|it| it.floor() as i32).into();

        let step_x = if dir.x > 0.0 { 1 } else if dir.x < 0.0 { -1 } else { 0 };
        let step_y = if dir.y > 0.0 { 1 } else if dir.y < 0.0 { -1 } else { 0 };
        let step_z = if dir.z > 0.0 { 1 } else if dir.z < 0.0 { -1 } else { 0 };

        let ox = self.position.x;
        let oy = self.position.y;
        let oz = self.position.z;

        let vx = voxel[0] as f32;
        let vy = voxel[1] as f32;
        let vz = voxel[2] as f32;

        let (mut t_max_x, t_delta_x) = if step_x != 0 {
            let next_boundary = if step_x > 0 { vx + 1.0 } else { vx };
            let t_max = (next_boundary - ox) / dir.x;
            (t_max, 1.0 / dir.x.abs())
        } else {
            (f32::INFINITY, f32::INFINITY)
        };
        let (mut t_max_y, t_delta_y) = if step_y != 0 {
            let next_boundary = if step_y > 0 { vy + 1.0 } else { vy };
            let t_max = (next_boundary - oy) / dir.y;
            (t_max, 1.0 / dir.y.abs())
        } else {
            (f32::INFINITY, f32::INFINITY)
        };
        let (mut t_max_z, t_delta_z) = if step_z != 0 {
            let next_boundary = if step_z > 0 { vz + 1.0 } else { vz };
            let t_max = (next_boundary - oz) / dir.z;
            (t_max, 1.0 / dir.z.abs())
        } else {
            (f32::INFINITY, f32::INFINITY)
        };

        let mut t = 0.0f32;
        while t <= max_distance {
            // step to next voxel boundary
            let hit_face;
            if t_max_x <= t_max_y && t_max_x <= t_max_z {
                voxel[0] += step_x;
                t = t_max_x;
                t_max_x += t_delta_x;
                hit_face = Some(if step_x > 0 { BlockFace::West } else { BlockFace::East });
            } else if t_max_y <= t_max_x && t_max_y <= t_max_z {
                voxel[1] += step_y;
                t = t_max_y;
                t_max_y += t_delta_y;
                hit_face = Some(if step_y > 0 { BlockFace::Down } else { BlockFace::Up });
            } else {
                voxel[2] += step_z;
                t = t_max_z;
                t_max_z += t_delta_z;
                hit_face = Some(if step_z > 0 { BlockFace::North } else { BlockFace::South });
            }

            if t > max_distance {
                break;
            }

            let block_id = world.get_block(voxel);
            if solid_block(block_id) {
                return Some((voxel, hit_face.unwrap()));
            }
        }

        None
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BlockFace {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl BlockFace {
    pub fn direction(&self) -> Vector3<i32> {
        match self {
            BlockFace::Down => vec3(0, -1, 0),
            BlockFace::Up => vec3(0, 1, 0),
            BlockFace::West => vec3(-1, 0, 0),
            BlockFace::East => vec3(1, 0, 0),
            BlockFace::South => vec3(0, 0, 1),
            BlockFace::North => vec3(0, 0, -1),
        }
    }
}