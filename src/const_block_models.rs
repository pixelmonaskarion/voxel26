use std::{fmt::Debug, fs::read, path::PathBuf};

use bespoke_engine::{binding::Descriptor, model::ToRaw, resource_compiler::Atlas, resource_loader::{ResourceConst, const_name}};
use bytemuck::{NoUninit, bytes_of};
use cgmath::{Vector3, vec2};
use serde::Deserialize;

#[repr(C)]
#[derive(NoUninit, Copy, Clone, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub normal: [f32; 4],
}

impl Vertex {
    #[allow(dead_code)]
    pub fn pos(&self) -> Vector3<f32> {
        return Vector3::new(self.position[0], self.position[1], self.position[2]);
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

pub fn parse_model(file_contents: Vec<u8>, block_atlas: &Atlas) -> BlockModel {
    let definition_json = String::from_utf8(file_contents).unwrap();
    let definition: ModelDefinition = serde_json::from_str(&definition_json).unwrap();
    let mut vertices = vec![];
    let mut indices = vec![];

    for element in definition.elements {
        let altas_section = |texture| {
            block_atlas.entries.get(texture).unwrap().clone()
        };
        let from: Vector3<f32> = Vector3::from(element.from).cast().unwrap();
        let to: Vector3<f32> = Vector3::from(element.to).cast().unwrap();
        let scale = 1.0/16.0;

        //north
        let section = altas_section(&element.faces.north.texture);
        let atlas_size_proportion = vec2(section.width as f32 / block_atlas.width as f32, section.height as f32 / block_atlas.height as f32);
        let atlas_start_x = section.x as f32 / block_atlas.width as f32;
        let atlas_start_y = section.y as f32 / block_atlas.width as f32;
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [atlas_start_x+element.faces.north.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.north.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [atlas_start_x+element.faces.north.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.north.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [atlas_start_x+element.faces.north.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.north.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [atlas_start_x+element.faces.north.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.north.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        }); 
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //south
        let section = altas_section(&element.faces.south.texture);
        let atlas_size_proportion = vec2(section.width as f32 / block_atlas.width as f32, section.height as f32 / block_atlas.height as f32);
        let atlas_start_x = section.x as f32 / block_atlas.width as f32;
        let atlas_start_y = section.y as f32 / block_atlas.width as f32;
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [atlas_start_x+element.faces.south.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.south.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [atlas_start_x+element.faces.south.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.south.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [atlas_start_x+element.faces.south.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.south.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [atlas_start_x+element.faces.south.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.south.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //east
        let section = altas_section(&element.faces.east.texture);
        let atlas_size_proportion = vec2(section.width as f32 / block_atlas.width as f32, section.height as f32 / block_atlas.height as f32);
        let atlas_start_x = section.x as f32 / block_atlas.width as f32;
        let atlas_start_y = section.y as f32 / block_atlas.width as f32;
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.east.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.east.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.east.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.east.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.east.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.east.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.east.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.east.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //west
        let section = altas_section(&element.faces.west.texture);
        let atlas_size_proportion = vec2(section.width as f32 / block_atlas.width as f32, section.height as f32 / block_atlas.height as f32);
        let atlas_start_x = section.x as f32 / block_atlas.width as f32;
        let atlas_start_y = section.y as f32 / block_atlas.width as f32;
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.west.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.west.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.west.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.west.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.west.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.west.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.west.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.west.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //up
        let section = altas_section(&element.faces.up.texture);
        let atlas_size_proportion = vec2(section.width as f32 / block_atlas.width as f32, section.height as f32 / block_atlas.height as f32);
        let atlas_start_x = section.x as f32 / block_atlas.width as f32;
        let atlas_start_y = section.y as f32 / block_atlas.width as f32;
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.up.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.up.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.up.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.up.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.up.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.up.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.up.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.up.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //down
        let section = altas_section(&element.faces.down.texture);
        let atlas_size_proportion = vec2(section.width as f32 / block_atlas.width as f32, section.height as f32 / block_atlas.height as f32);
        let atlas_start_x = section.x as f32 / block_atlas.width as f32;
        let atlas_start_y = section.y as f32 / block_atlas.width as f32;
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.down.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.down.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.down.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.down.uv[1] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.down.uv[0] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.down.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [atlas_start_x+element.faces.down.uv[2] as f32 * scale*atlas_size_proportion.x, atlas_start_y+element.faces.down.uv[3] as f32 * scale*atlas_size_proportion.y, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);

    }

    BlockModel { vertices, indices }
}

pub fn generate_models(rg: &mut bespoke_engine::resource_loader::ResourceGenerator, files: Vec<PathBuf>, block_atlas: &Atlas) {
    for file in files {
        let file_contents = read(&file).unwrap();
        let BlockModel { vertices, indices } = parse_model(file_contents, &block_atlas);
        let vertex_qualified_name = "crate::const_block_models::Vertex";
        let const_block_model_qualified_name = "crate::const_block_models::ConstBlockModel";
        let block_model_trait_qualified_name = "dyn crate::const_block_models::BlockModelTrait";
        let vertices_string: String = vertices.into_iter().map(|vertex| format!("{vertex_qualified_name} {{ position: {:?}, color: {:?}, normal: {:?} }}, ", vertex.position, vertex.color, vertex.normal)).collect();
        let indices_string: String = indices.into_iter().map(|index| format!("{index}, ")).collect();
        let model_string = format!("&{const_block_model_qualified_name} {{ vertices: [{vertices_string}], indices: [{indices_string}] }}");
        rg.consts.push(ResourceConst { name: format!("GENERATED_{}_BLOCK_MODEL", const_name(file.file_prefix().unwrap().to_str().unwrap().to_string())), rtype: format!("&'static {block_model_trait_qualified_name}"), value: model_string });
    }
} 

#[derive(Deserialize)]
struct ModelDefinition {
    elements: Vec<ModelElement>
}

#[derive(Deserialize)]
struct ModelElement {
    from: [i32; 3],
    to: [i32; 3],
    faces: ModelFaces,
}

#[derive(Deserialize)]
struct ModelFaces {
    north: ModelFace,
    east: ModelFace,
    south: ModelFace,
    west: ModelFace,
    up: ModelFace,
    down: ModelFace,
}

#[derive(Deserialize)]
struct ModelFace {
    uv: [i32; 4],
    texture: String,
}

#[derive(Deserialize)]
#[allow(unused)]
struct ModelRotation {
    angle: f32,
    axis: String,
    origin: [f32; 3],
}