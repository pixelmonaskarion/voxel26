use bespoke_engine::{binding::{Binding, Descriptor, create_layout}, culling::AABB, model::{Model, ToRaw}, shader::{Shader, ShaderConfig}, surface_context::SurfaceCtx, texture::Texture, window::BasicVertex};
use bytemuck::bytes_of;
use cgmath::Vector3;
use wgpu::{Buffer, BufferUsages, wgt::BufferDescriptor};

use crate::{chunk::ChunkManager, game::ScreenInfo, inventory::{InventoryItemStack, ItemAtlas}};

pub struct Entity {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
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

    pub fn update(&mut self, world: &ChunkManager, delta_time: f64) {
        self.position += self.velocity * delta_time as f32;
    }

    pub fn shader_instance(&self, item_atlas: &ItemAtlas) -> Option<Vec<u8>> {
        match &self.entity_type {
            TypedEntity::Item { stack } => {
                let atlas_subsection = item_atlas.subsection_for_position(stack.atlas_coordinates);
                Some(ItemInstance { position: self.position.extend(1.0).into(), texture_offsets: [0.0; 4], atlas_subsection }.to_raw())
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
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let size = 0.2;
        let item_model = Model::new(vec![
            BasicVertex { position: [-size, -size, 0.0], tex_coords: [0.0, 1.0] },
            BasicVertex { position: [-size, size, 0.0], tex_coords: [0.0, 0.0] },
            BasicVertex { position: [size, -size, 0.0], tex_coords: [1.0, 1.0] },
            BasicVertex { position: [size, size, 0.0], tex_coords: [1.0, 0.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB { dimensions: [1.0, 1.0, 0.0] }, surface_ctx.device());
        let item_shader = Shader::new("res/shaders/item_entity.wgsl", surface_ctx.device(), vec![surface_ctx.config().format], vec![&create_layout::<ScreenInfo>(surface_ctx.device()), &create_layout::<[[f32; 4]; 4]>(surface_ctx.device()), &create_layout::<[[f32; 4]; 4]>(surface_ctx.device()), &create_layout::<Texture>(surface_ctx.device())], vec![&ScreenInfo::shader_type(), &<[[f32; 4]; 4]>::shader_type(), &<[[f32; 4]; 4]>::shader_type(), &Texture::shader_type()], vec![BasicVertex::desc(), ItemInstance::desc()], ShaderConfig::default());
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
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
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
        }
    }
}