use std::time::Duration;

use bespoke_engine::{binding::{Binding, Descriptor, create_layout}, culling::AABB, model::{Model, ToRaw}, shader::{Shader, ShaderInit}, surface_context::SurfaceCtx, texture::{Texture, TextureLayoutConfig}, window::BasicVertex};
use bytemuck::bytes_of;
use cgmath::{Vector3, vec3};
use wgpu::{Buffer, BufferUsages, TextureFormat, wgt::BufferDescriptor};

use crate::{RES_SHADERS_ITEM_ENTITY_WGSL, chunk::ChunkManager, game::ScreenInfo, inventory::{InventoryItemStack, ItemAtlas}, util};

pub struct Entity {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub time_alive: Duration,
    pub entity_type: TypedEntity,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub enum EntityType {
    Item,
    Marker,
}

impl Entity {
    pub fn id(&self) -> EntityType {
        match &self.entity_type {
            TypedEntity::Item { .. } => EntityType::Item,
            TypedEntity::Marker => EntityType::Marker
        }
    }

    pub fn update(&mut self, world: &ChunkManager, delta_time: Duration) {
        self.time_alive += delta_time;
        self.velocity.x *= 0.9;
        self.velocity.z *= 0.9;
        self.velocity -= vec3(0.0, 20.0 * delta_time.as_secs_f32(), 0.0);
        let delta = self.velocity * delta_time.as_secs_f32();
        let x_steps = (delta.x.abs()/0.5).ceil();
        let colliding = |position: Vector3<f32>| -> bool {
            util::colliding_world(world, position, vec3(0.15, 0.15, 0.15), vec3(-0.15, -0.15, -0.15))
        };
        for _ in 0..x_steps as i32 {
            self.position.x += delta.x/x_steps;
            if colliding(self.position) {
                self.position.x -= delta.x/x_steps;
                self.velocity.x = 0.0;
            }
        }
        let z_steps = (delta.z.abs()/0.5).ceil();
        for _ in 0..z_steps as i32 {
            self.position.z += delta.z/z_steps;
            if colliding(self.position) {
                self.position.z -= delta.z/z_steps;
                self.velocity.z = 0.0;
            }
        }
        let y_steps = (delta.y.abs()/0.5).ceil();
        for _ in 0..y_steps as i32 {
            self.position.y += delta.y/y_steps;
            if colliding(self.position) {
                self.position.y -= delta.y/y_steps;
                self.velocity.y = 0.0;
            }
        }

    }

    pub fn shader_instance(&self, item_atlas: &ItemAtlas) -> Option<Vec<u8>> {
        match &self.entity_type {
            TypedEntity::Item { stack } => {
                let atlas_subsection = item_atlas.subsection_for_position(stack.atlas_coordinates);
                Some(ItemInstance { position: (self.position+vec3(0.0, (self.time_alive.as_secs_f32().sin()+1.0)*0.05, 0.0)).extend(1.0).into(), texture_offsets: [0.0; 4], atlas_subsection }.to_raw())
            },
            TypedEntity::Marker => {
                None
            }
        }
    }
}

pub enum TypedEntity {
    Item {
        stack: InventoryItemStack,
    },
    Marker
}

pub struct EntityRenderManager<'a> {
    pub item_model: Model,
    pub item_shader: Shader<'a>,
    pub instance_buffer: Buffer,
}

impl <'a> EntityRenderManager<'a> {
    pub fn new(surface_ctx: &dyn SurfaceCtx, deferred_formats: Vec<TextureFormat>) -> Self {
        let size = 0.2;
        let item_model = Model::new(vec![
            BasicVertex { position: [-size, -size, 0.0], tex_coords: [0.0, 1.0] },
            BasicVertex { position: [-size, size, 0.0], tex_coords: [0.0, 0.0] },
            BasicVertex { position: [size, -size, 0.0], tex_coords: [1.0, 1.0] },
            BasicVertex { position: [size, size, 0.0], tex_coords: [1.0, 0.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB { dimensions: [1.0, 1.0, 0.0] }, surface_ctx.device());
        let item_shader = Shader::new(ShaderInit { resource: RES_SHADERS_ITEM_ENTITY_WGSL, formats: deferred_formats.clone(), binding_layouts: vec![create_layout::<ScreenInfo>((), surface_ctx.device()), create_layout::<Texture>(TextureLayoutConfig::default(), surface_ctx.device())], shader_types: vec![ScreenInfo::shader_type(()), Texture::shader_type(TextureLayoutConfig::default())], vertex_buffers: vec![BasicVertex::desc(), ItemInstance::desc()], ..Default::default() }, surface_ctx.device());
        let instance_buffer = surface_ctx.device().create_buffer(&BufferDescriptor {
            label: Some("Entity Instance Buffer"),
            mapped_at_creation: false,
            size: 1048576,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        Self {
            item_model,
            item_shader,
            instance_buffer,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct ItemInstance {
    position: [f32; 4],
    texture_offsets: [f32; 4],
    atlas_subsection: [f32; 4],
}

impl ToRaw for ItemInstance {
    fn to_raw(&self) -> Vec<u8> {
        bytes_of(self).to_vec()
    }
}

impl Descriptor for ItemInstance {
    fn desc<'a>() -> Option<wgpu::VertexBufferLayout<'a>> {
        use std::mem;
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ItemInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        })
    }
}