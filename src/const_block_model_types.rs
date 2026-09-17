use std::fmt::Debug;

use bespoke_engine::{binding::Descriptor, model::ToRaw};
use bytemuck::{NoUninit, bytes_of};
use glam::{Vec3, vec3};

#[repr(C)]
#[derive(NoUninit, Copy, Clone, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub normal: [f32; 4],
}

impl Vertex {
    #[allow(dead_code)]
    pub fn pos(&self) -> Vec3 {
        return vec3(self.position[0], self.position[1], self.position[2]);
    }
}

impl Descriptor for Vertex {
    fn desc<'a>() -> Option<wgpu::VertexBufferLayout<'a>> {
        use std::mem;
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        })
    }
}

impl ToRaw for Vertex {
    fn to_raw(&self) -> Vec<u8> {
        bytes_of(self).to_vec()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BlockModel {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConstBlockModel<const V: usize, const I: usize> {
    pub vertices: [Vertex; V],
    pub indices: [u32; I],
}

pub trait BlockModelTrait: Debug + Sync {
    fn vertices(&self) -> &[Vertex];
    fn indices(&self) -> &[u32];
}

impl<const V: usize, const I: usize> BlockModelTrait for ConstBlockModel<V, I> {
    fn vertices(&self) -> &[Vertex] { &self.vertices }
    fn indices(&self) -> &[u32] { &self.indices }
}

impl<const V: usize, const I: usize> AsRef<dyn BlockModelTrait + 'static> for ConstBlockModel<V, I> {
    fn as_ref(&self) -> &(dyn BlockModelTrait + 'static) {
        self
    }
}