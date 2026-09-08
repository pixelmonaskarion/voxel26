use std::{hash::Hash, time::Duration};

use bespoke_engine::camera::Camera;
use glam::{IVec3, Vec3, ivec3, vec3};
use ordered_float::OrderedFloat;
use rustc_hash::FxHashMap;

use crate::{blocks::Block, chunk::ChunkManager, inventory::Inventory, registries::{Registries, TagID}};

pub struct Player {
    pub camera: Camera,
    pub position: Vec3,
    pub velocity: Vec3,
    pub time_since_ground: Duration,
    pub movement_mode: i32,
    pub break_cooldown: Duration,
    pub break_progress: Duration,
    pub break_position: Option<IVec3>,

    pub inventory: Inventory,
    pub attributes: FxHashMap<EntityAttribute, f32>,
    pub health: f32,
}

impl Player {
    pub fn new(position: Vec3, camera: Camera, registries: &Registries) -> Self {
        let mut _self = Self {
            camera,
            position,
            velocity: vec3(0.0, 0.0, 0.0),
            time_since_ground: Duration::new(2, 0),
            movement_mode: 0,
            break_cooldown: Duration::ZERO,
            break_progress: Duration::ZERO,
            break_position: None,
            inventory: Inventory::empty_size(4*9, registries),
            health: 20.0,
            attributes: FxHashMap::from_iter([(EntityAttribute::BlockBreakSpeed, 1.0)])
        };
        _self
    }

    pub fn move_player(&mut self, delta: Vec3, world: &ChunkManager, registries: &Registries) {
        let x_steps = (delta.x.abs()/0.5).ceil();
        for _ in 0..x_steps as i32 {
            self.position.x += delta.x/x_steps;
            if self.colliding_world(world, registries) {
                self.position.x -= delta.x/x_steps;
                self.velocity.x = 0.0;
            }
        }
        let z_steps = (delta.z.abs()/0.5).ceil();
        for _ in 0..z_steps as i32 {
            self.position.z += delta.z/z_steps;
            if self.colliding_world(world, registries) {
                self.position.z -= delta.z/z_steps;
                self.velocity.z = 0.0;
            }
        }
        let y_steps = (delta.y.abs()/0.5).ceil();
        for _ in 0..y_steps as i32 {
            self.position.y += delta.y/y_steps;
            if self.colliding_world(world, registries) {
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

    pub fn block_break_modifier(&mut self, target: Block, registries: &Registries) -> f32 {
        let mut attribute = *self.attributes.get(&EntityAttribute::BlockBreakSpeed).unwrap_or(&1.0);
        let conditional_modifier = registries.item_registry.get_item(self.inventory.selected_item().stack.item).attribute_modifiers.get(&EntityAttribute::BlockBreakSpeed).cloned().unwrap_or_default();
        if match conditional_modifier.condition {
            AttributeModifierCondition::Always => true,
            AttributeModifierCondition::TargetInTag(tag_id) => registries.tag_registry.get_block_tag(tag_id).entries.contains(&target.id),
        } {
            attribute = conditional_modifier.modifier.modify(attribute);
        }
        attribute
    }
    
    pub fn add_break_progress(&mut self, delta: Duration, target: Block, registries: &Registries) {
        let attribute = self.block_break_modifier(target, registries);
        self.break_progress += delta.mul_f32(attribute);
    }

    pub fn colliding_world(&self, world: &ChunkManager, registries: &Registries) -> bool {
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
            let block_position = position.floor().as_ivec3().into();
            let block = world.get_block(block_position);
            if registries.block_registry.get_block(&block).solid {
                return true;
            }
        }
        return false;
    }

    pub fn raycast(&self, direction: Vec3, max_distance: f32, world: &ChunkManager, registries: &Registries) -> Option<([i32;3], BlockFace)> {
        if direction.length_squared() == 0.0 {
            return None;
        }
        let dir = direction.normalize();
        let mut voxel: [i32; 3] = self.position.floor().as_ivec3().into();

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
            if registries.block_registry.get_block(&block_id).solid {
                return Some((voxel, hit_face.unwrap()));
            }
        }

        None
    }
}

#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Clone, Copy, Debug, rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)]
#[rkyv(
    compare(PartialEq),
    derive(Hash, PartialEq, Eq),
)]
pub enum EntityAttribute {
    BlockBreakSpeed,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Copy, Debug, rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)]
pub enum AttributeModifier {
    Multiply(f32),
    Add(f32),
}

impl Eq for AttributeModifier {}
impl Hash for AttributeModifier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            AttributeModifier::Add(v) => {
                0_usize.hash(state);
                OrderedFloat(*v).hash(state);
            },
            AttributeModifier::Multiply(v) => {
                1_usize.hash(state);
                OrderedFloat(*v).hash(state);
            }
        }
    }
}

impl Default for AttributeModifier {
    fn default() -> Self {
        Self::Add(0.0.into())
    }
}

impl AttributeModifier {
    pub fn modify(&self, value: f32) -> f32 {
        match self {
            AttributeModifier::Add(m) => value + *m,
            AttributeModifier::Multiply(m) => value * *m
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AttributeModifierCondition {
    Always,
    TargetInTag(TagID),
}

impl Default for AttributeModifierCondition {
    fn default() -> Self {
        Self::Always
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
    pub fn direction(&self) -> IVec3 {
        match self {
            BlockFace::Down => ivec3(0, -1, 0),
            BlockFace::Up => ivec3(0, 1, 0),
            BlockFace::West => ivec3(-1, 0, 0),
            BlockFace::East => ivec3(1, 0, 0),
            BlockFace::South => ivec3(0, 0, 1),
            BlockFace::North => ivec3(0, 0, -1),
        }
    }
}