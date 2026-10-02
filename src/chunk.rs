use std::{collections::{HashMap, VecDeque}, hash::{DefaultHasher, Hash, Hasher}, ops::{AddAssign, Mul, Range}, sync::{Arc, mpsc}, time::{Duration, SystemTime}};

use bespoke_engine::{binding::UniformBinding, camera::Camera, culling::{self, AABB}, model::Render, shader::Shader, surface_context::SurfaceCtx};
use glam::{DVec2, IVec2, IVec3, Mat4, UVec3, Vec3, dvec2, ivec2, ivec3, uvec3, vec3};
use noise::{NoiseFn, Perlin};
use ordered_float::OrderedFloat;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use rustc_hash::FxHashMap;
use serde_inline_default::serde_inline_default;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Device, RenderPass, ShaderStages};
use itertools::Itertools;

use crate::{BLOCK_ATLAS_PNG_HEIGHT, BLOCK_ATLAS_PNG_WIDTH, blocks::{self, AIR, Block, BlockID, DIRT, GRASS, NOT_RENDERED_LAYER, SOLID_LAYER, STONE, WATER}, const_block_model_types::{BlockModel, BlockModelTrait, Vertex}, entity::{Entity, EntityRenderManager, EntityType}, features::{Feature, FeatureType, bush::BushFeature, caves::WormCavesFeature, ore::OreFeature, tree::TreeFeature}, lighting::{BlockLightingData, SkyLightingData, TopBlockData}, registries::Registries, util::{chunk_for_block_position, neighbors}};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde_inline_default]
pub struct Chunk {
    pub data: ChunkData,
    #[serde(skip)]
    #[serde_inline_default(None)]
    pub model: Option<ChunkModel>,
    #[serde(skip)]
    #[serde_inline_default(false)]
    pub creating_model: bool,
    #[serde(skip)]
    #[serde_inline_default(false)]
    pub creating_blocks: bool,
    #[serde(skip)]
    pub needed_chunk_updates: Vec<NeededChunkUpdate>,
    #[serde_inline_default(0)]
    pub lod: i32,
    #[serde(skip)]
    #[serde_inline_default(BlockLightingData::new())]
    pub lighting: BlockLightingData,
    #[serde(skip)]
    #[serde_inline_default(SkyLightingData::new())]
    pub skylight: SkyLightingData,
}

pub const CHUNK_BOUNDING_BOX: AABB = AABB { dimensions: [CHUNK_SIZE as f32 / 2.0; 3] };

#[derive(serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct ChunkData {
    pub blocks: Vec<BlockID>,
    pub generated_blocks: bool,
    pub entities: FxHashMap<EntityType, Vec<Entity>>,
    pub block_data: FxHashMap<UVec3, Vec<u8>>,
    pub chunk_coords: IVec3,
}

pub struct ChunkModel {
    // vertices: Buffer,
    buffer: Buffer,
    layers: Vec<ChunkModelLayer>,
    lighting_buffer: Buffer,
    lighting_bind_group: BindGroup,
    skylight_buffer: Buffer,
    skylight_bind_group: BindGroup,
    needs_lighting_update: bool,
    chunk_position_uniform: UniformBinding<[f32; 3]>,
}

impl ChunkModel {
    pub fn num_vertices(&self, layer: usize) -> u64 {
        self.layers[layer].num_vertices as u64
    }

    pub fn num_indices(&self, layer: usize) -> u64 {
        self.layers[layer].num_indices as u64
    }

    pub fn num_modeled_vertices(&self, layer: usize) -> u64 {
        self.layers[layer].num_modeled_vertices as u64
    }

    pub fn num_modeled_indices(&self, layer: usize) -> u64 {
        self.layers[layer].num_modeled_indices as u64
    }

    pub fn vertices_range(&self, layer: usize) -> Range<u64> {
        let start = if layer == 0 {
            self.actual_indices_range().end
        } else {
            self.vertices_range(layer-1).end
        };
        start..start+(self.layers[layer].num_vertices*size_of::<Vertex>()) as u64
    }

    pub fn indices_range(&self, layer: usize) -> Range<u64> {
        0..(self.layers[layer].num_indices*size_of::<u32>()) as u64
    }

    pub fn actual_indices_range(&self) -> Range<u64> {
        0..(0..self.layers.len()).map(|layer| self.indices_range(layer).end).max().unwrap_or(0)
    }

    pub fn modeled_vertices(&self, layer: usize) -> Range<u64> {
        let start = if layer == 0 {
            self.vertices_range(self.layers.len()-1).end
        } else {
            self.modeled_indices(layer-1).end
        };
        start..start+(self.layers[layer].num_modeled_vertices*size_of::<Vertex>()) as u64
    }

    pub fn modeled_indices(&self, layer: usize) -> Range<u64> {
        self.modeled_vertices(layer).end..(self.layers[layer].num_modeled_indices*size_of::<u32>()) as u64
    }

    pub fn visible(&self) -> bool {
        self.layers.iter().any(|it| it.num_indices > 0 || it.num_modeled_indices > 0)
    }
}

pub struct ChunkModelLayer {
    num_vertices: usize,
    num_indices: usize,
    num_modeled_vertices: usize,
    num_modeled_indices: usize,
}

#[derive(Clone)]
pub struct NeededChunkUpdate {
    pub relative_chunk_pos: IVec3,
    pub synchronous: bool,
}

pub const CHUNK_SIZE: u32 = 32;

#[inline(always)]
pub fn index_in_chunk(x: u32, y: u32, z: u32) -> usize {
    (y * CHUNK_SIZE * CHUNK_SIZE + x * CHUNK_SIZE + z) as usize
}

#[allow(unused)]
impl Chunk {
    pub fn new(chunk_coords: IVec3) -> Self {
        let mut _self = Self {
            data: ChunkData {
                blocks: vec![0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize],
                generated_blocks: false,
                entities: FxHashMap::default(),
                block_data: FxHashMap::default(),
                chunk_coords,
            },
            creating_model: false,
            creating_blocks: false,
            model: None,
            needed_chunk_updates: vec![],
            lod: 0,
            lighting: BlockLightingData::new(),
            skylight: SkyLightingData::new(),
        };
        _self
    }

    pub fn set_block(&mut self, local_coords: UVec3, block: BlockID, state: Option<Vec<u8>>, update_synchronously: bool, registries: &Registries) {
        self.data.blocks[index_in_chunk(local_coords.x, local_coords.y, local_coords.z)] = block;
        if let Some(state) = state {
            self.data.block_data.insert(local_coords, state);
        } else {
            self.data.block_data.remove(&local_coords);
        }
        self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(0, 0, 0), synchronous: update_synchronously });
        if local_coords[0] == 0 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(-1, 0, 0), synchronous: update_synchronously });
        }
        if local_coords[1] == 0 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(0, -1, 0), synchronous: update_synchronously });
        }
        if local_coords[2] == 0 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(0, 0, -1), synchronous: update_synchronously });
        }
        if local_coords[0] == CHUNK_SIZE-1 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(1, 0, 0), synchronous: update_synchronously });
        }
        if local_coords[1] == CHUNK_SIZE-1 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(0, 1, 0), synchronous: update_synchronously });
        }
        if local_coords[2] == CHUNK_SIZE-1 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(0, 0, 1), synchronous: update_synchronously });
        }
    }

    pub fn render<'s: 'b, 'b>(&'s self, render_pass: &mut RenderPass<'b>, layer_index: usize, entity_render_manager: &'b EntityRenderManager, registries: &Registries, chunk_shader: &'b Shader<'b>, surface_ctx: &dyn SurfaceCtx) {
        chunk_shader.bind(render_pass);
        render_pass.set_bind_group(1, &registries.block_atlas_texture.binding, &[]);
        if let Some(model) = &self.model {
            render_pass.set_bind_group(2, &model.lighting_bind_group, &[]);
            render_pass.set_bind_group(3, &model.chunk_position_uniform.binding, &[]);
            render_pass.set_bind_group(4, &model.skylight_bind_group, &[]);
            if model.num_vertices(layer_index) > 0 {
                render_pass.set_vertex_buffer(0, model.buffer.slice(model.vertices_range(layer_index)));
                render_pass.set_index_buffer(model.buffer.slice(model.indices_range(layer_index)), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model.num_indices(layer_index) as u32, 0, 0..1);
            }
            if model.num_modeled_vertices(layer_index) > 0 {
                render_pass.set_vertex_buffer(0, model.buffer.slice(model.modeled_vertices(layer_index)));
                render_pass.set_index_buffer(model.buffer.slice(model.modeled_indices(layer_index)), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model.num_modeled_indices(layer_index) as u32, 0, 0..1);
            }
            if layer_index == 0 {
                for (entity_type, entities) in &self.data.entities {
                    if entities.len() == 0 {
                        continue;
                    }
                    match *entity_type {
                        EntityType::Item => {
                            entity_render_manager.item_shader.bind(render_pass);
                            render_pass.set_bind_group(1, &registries.item_atlas_registry.texture.binding, &[]);
                        }
                        _ => {}
                    }
                    let max_batch_size = entity_render_manager.instance_buffer.size() as usize;
                    let mut instance_size = 0;
                    let instances = entities.iter().flat_map(|it| {
                        let instance = it.shader_instance(&registries.item_atlas_registry);
                        if let Some(instance) = &instance {
                            instance_size = instance.len();
                        }
                        instance
                    }).flatten().collect::<Vec<u8>>();
                    if instance_size > 0 && instances.len() > 0 {
                        for batch in instances.chunks((max_batch_size/instance_size)*instance_size) {
                            surface_ctx.queue().write_buffer(&entity_render_manager.instance_buffer, 0, batch);
                            entity_render_manager.item_model.render_instances(render_pass, &entity_render_manager.instance_buffer, 0..(batch.len()/instance_size) as u32);
                        }
                    }
                }
            }
            // chunk_shader.bind(render_pass);
            // render_pass.set_bind_group(1, &registries.block_atlas_texture.binding, &[]);
            // if model.num_transparent_vertices > 0 {
            //     render_pass.set_vertex_buffer(0, model.buffer.slice((model.num_vertices*size_of::<Vertex>()) as u64..(model.num_vertices*size_of::<Vertex>()+model.num_transparent_vertices*size_of::<Vertex>()) as u64));
            //     render_pass.set_index_buffer(model.buffer.slice(((model.num_vertices+model.num_transparent_vertices)*size_of::<Vertex>()) as u64..((model.num_vertices+model.num_transparent_vertices)*size_of::<Vertex>()+model.num_transparent_indices*size_of::<u32>()) as u64), wgpu::IndexFormat::Uint32);
            //     render_pass.draw_indexed(0..model.num_transparent_indices as u32, 0, 0..1);
            // }
        }
    }

    pub fn visible(&self, camera: &Camera, view_proj: Mat4) -> bool {
        let transform = Mat4::from_translation((self.data.chunk_coords.as_vec3()+vec3(0.5, 0.5, 0.5)) * CHUNK_SIZE as f32);
        self.model.as_ref().is_some_and(|it| it.visible()) && !culling::culled(&CHUNK_BOUNDING_BOX, transform, camera, view_proj)
    }

    pub fn add_entity(&mut self, entity: Entity) {
        let id = entity.id();
        if !self.data.entities.contains_key(&id) {
            self.data.entities.insert(id, vec![]);
        }
        self.data.entities.get_mut(&id).unwrap().push(entity);
    }
}

struct GenerateChunkBlocksRequest {
    chunk_position: IVec3,
    player_position: [f32; 3],
}

struct GenerateChunkBlocksResponse {
    chunk_position: IVec3,
    chunk_blocks: Vec<BlockID>,
}

struct GenerateChunkMeshRequest {
    chunk_position: IVec3,
    player_position: [f32; 3],
    chunk_blocks: Vec<BlockID>,
    cpx: Option<Vec<BlockID>>,
    cnx: Option<Vec<BlockID>>,
    cpy: Option<Vec<BlockID>>,
    cny: Option<Vec<BlockID>>,
    cpz: Option<Vec<BlockID>>,
    cnz: Option<Vec<BlockID>>,
    lighting: LightingBufferOrData<BlockLightingData>,
    skylight: LightingBufferOrData<SkyLightingData>,
}

enum LightingBufferOrData<T> {
    Buffer(Buffer),
    Data(T),
}

struct GenerateChunkModelResponse {
    chunk_position: IVec3,
    chunk_model: Option<ChunkModel>,
    lod: i32,
}

pub struct ChunkManager {
    chunks: ChunkMap,
    pub pending_requests: i32,
    gen_blocks_tx: mpsc::Sender<GenerateChunkBlocksRequest>,
    gen_blocks_rx: mpsc::Receiver<GenerateChunkBlocksResponse>,
    gen_model_tx: mpsc::Sender<GenerateChunkMeshRequest>,
    gen_model_rx: mpsc::Receiver<GenerateChunkModelResponse>,
    skylights: SkylightMap,
}
#[allow(unused)]
impl ChunkManager {
    pub fn new(seed: u32, registries: Arc<Registries>, surface_ctx: &dyn SurfaceCtx) -> Self {
        let (gen_blocks_req_tx, gen_blocks_req_rx) = mpsc::channel::<GenerateChunkBlocksRequest>();
        let (gen_blocks_res_tx, gen_blocks_res_rx) = mpsc::channel();
        let (gen_model_req_tx, gen_model_req_rx) = mpsc::channel::<GenerateChunkMeshRequest>();
        let (gen_model_res_tx, gen_model_res_rx) = mpsc::channel();
        let device = surface_ctx.device_arc().clone();
        tokio::spawn(async move {
            let mut n = 0;
            let mut t = Duration::ZERO;
            let mut queue: Vec<GenerateChunkBlocksRequest> = Vec::new();
            let mut player_position: [f32; 3];
            let mut resources = ChunkBlockGeneratorResources::new(seed);
            loop {
                match gen_blocks_req_rx.recv() {
                    Ok(req) => {
                        player_position = req.player_position;
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2_by_world(&it.chunk_position, player_position) < Self::chunk_distance2_by_world(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
                    },
                    Err(_) => {
                        break;
                    }
                }
                while let Some(req) = queue.pop() {
                    let start = SystemTime::now();
                    let Ok(_) = gen_blocks_res_tx.send(Self::generate_blocks_req(req, &mut resources, None::<fn(Vec<f64>)>)) else {
                        println!("ending block generator");
                        return;
                    };
                    t += SystemTime::now().duration_since(start).unwrap();
                    n += 1;
                    if n % 1000 == 999 {
                        let avg = t/n;
                        println!("avg gen blocks: {avg:?}");
                    }
                    while let Ok(req) = gen_blocks_req_rx.try_recv() {
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2_by_world(&it.chunk_position, player_position) < Self::chunk_distance2_by_world(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
                    }
                }
            }
        });
        tokio::spawn(async move {
            let mut n = 0;
            let mut t = Duration::ZERO;
            let mut queue: Vec<GenerateChunkMeshRequest> = Vec::new();
            let mut player_position: [f32; 3];
            loop {
                match gen_model_req_rx.recv() {
                    Ok(req) => {
                        player_position = req.player_position;
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2_by_world(&it.chunk_position, player_position) > Self::chunk_distance2_by_world(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
                    },
                    Err(_) => {
                        break;
                    }
                }
                while let Some(req) = queue.pop() {
                    let start = SystemTime::now();
                    let Ok(_) = gen_model_res_tx.send(Self::generate_mesh_req(req, &device, &registries)) else {
                        println!("ending mesh generator");
                        return
                    };
                    t += SystemTime::now().duration_since(start).unwrap();
                    n += 1;
                    if n % 1000 == 999 {
                        let avg = t/n;
                        println!("avg gen mesh: {avg:?}");
                        t = Duration::ZERO;
                        n = 0;
                    }
                    while let Ok(req) = gen_model_req_rx.try_recv() {
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2_by_world(&it.chunk_position, player_position) > Self::chunk_distance2_by_world(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
                    }
                }
            }
        });
        Self {
            chunks: ChunkMap::new(),
            skylights: SkylightMap::new(surface_ctx.device()),
            gen_blocks_tx: gen_blocks_req_tx,
            gen_blocks_rx: gen_blocks_res_rx,
            gen_model_tx: gen_model_req_tx,
            gen_model_rx: gen_model_res_rx,
            pending_requests: 0,
        }
    }

    pub fn poll_channels(&mut self, player_position: [f32; 3], registries: &Registries, device: &Device) {
        while let Ok(res) = self.gen_blocks_rx.try_recv() {
            self.pending_requests -= 1;
            if let Some(chunk) = self.chunks.get_mut(res.chunk_position) {
                chunk.data.blocks = res.chunk_blocks;
                chunk.data.generated_blocks = true;
                chunk.creating_blocks = false;
                chunk.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(0, 0, 0), synchronous: false });
                chunk.needed_chunk_updates.extend(neighbors(IVec3::ZERO).into_iter().map(|it| NeededChunkUpdate { relative_chunk_pos: it.into(), synchronous: false }));
                self.update_skylight_map_for_chunk(res.chunk_position, registries);
                self.update_lighting_around(res.chunk_position, registries);
            }
        }
        while let Ok(res) = self.gen_model_rx.try_recv() {
            self.pending_requests -= 1;
            if let Some(chunk) = self.chunks.get_mut(res.chunk_position) {
                chunk.model = res.chunk_model;
                chunk.creating_model = false;
                chunk.lod = res.lod;
                // chunk.transparency_model = res.transparency_model;
            }
        }
    }

    pub fn generate_blocks(&mut self, chunk_position: IVec3, player_position: [f32; 3]) {
        if let Some(chunk) = self.chunks.get_mut(chunk_position) {
            self.pending_requests += 1;
            chunk.creating_blocks = true;
            self.gen_blocks_tx.send(GenerateChunkBlocksRequest {
                chunk_position,
                player_position,
            }).unwrap();
        }
    }

    pub fn generate_model(&mut self, chunk_position: IVec3, player_position: [f32; 3]) {
        if let Some(chunk) = self.chunks.get_mut(chunk_position) {
            self.pending_requests += 1;
            chunk.creating_model = true;
            self.gen_model_tx.send(GenerateChunkMeshRequest {
                chunk_position,
                player_position,
                chunk_blocks: chunk.data.blocks.clone(),
                lighting: chunk.model.as_ref().map(|it| if it.needs_lighting_update { LightingBufferOrData::Data(chunk.lighting.clone()) } else { LightingBufferOrData::Buffer(it.lighting_buffer.clone()) }).unwrap_or(LightingBufferOrData::Data(chunk.lighting.clone())),
                skylight: chunk.model.as_ref().map(|it| if it.needs_lighting_update { LightingBufferOrData::Data(chunk.skylight.clone()) } else { LightingBufferOrData::Buffer(it.skylight_buffer.clone()) }).unwrap_or(LightingBufferOrData::Data(chunk.skylight.clone())),
                cpx: self.chunks.get(ivec3(chunk_position[0]+1, chunk_position[1], chunk_position[2])).map(|it| it.data.blocks.clone()),
                cnx: self.chunks.get(ivec3(chunk_position[0]-1, chunk_position[1], chunk_position[2])).map(|it| it.data.blocks.clone()),
                cpy: self.chunks.get(ivec3(chunk_position[0], chunk_position[1]+1, chunk_position[2])).map(|it| it.data.blocks.clone()),
                cny: self.chunks.get(ivec3(chunk_position[0], chunk_position[1]-1, chunk_position[2])).map(|it| it.data.blocks.clone()),
                cpz: self.chunks.get(ivec3(chunk_position[0], chunk_position[1], chunk_position[2]+1)).map(|it| it.data.blocks.clone()),
                cnz: self.chunks.get(ivec3(chunk_position[0], chunk_position[1], chunk_position[2]-1)).map(|it| it.data.blocks.clone()),
            }).unwrap();
        }
    }

    pub fn generate_model_and_surroundings(&mut self, chunk_position: IVec3, player_position: [f32; 3]) {
        self.generate_model(chunk_position, player_position);
        for position in neighbors(chunk_position) {
            self.generate_model(position, player_position);
        }
    }

    pub fn generate_model_and_surroundings_now(&mut self, chunk_position: IVec3, player_position: [f32; 3], registries: &Registries, surface_ctx: &dyn SurfaceCtx) {
        self.generate_model_now(chunk_position, player_position, registries, surface_ctx);
        for position in neighbors(chunk_position) {
            self.generate_model_now(position, player_position, registries, surface_ctx);
        }
    }

    pub fn generate_model_now(&mut self, chunk_position: IVec3, player_position: [f32; 3], registries: &Registries, surface_ctx: &dyn SurfaceCtx) {
        let time = SystemTime::now();
        if let Some(chunk) = self.chunks.get(chunk_position) {
            let req = GenerateChunkMeshRequest {
                chunk_position,
                player_position,
                chunk_blocks: chunk.data.blocks.clone(),
                lighting: chunk.model.as_ref().map(|it| if it.needs_lighting_update { LightingBufferOrData::Data(chunk.lighting.clone()) } else { LightingBufferOrData::Buffer(it.lighting_buffer.clone()) }).unwrap_or(LightingBufferOrData::Data(chunk.lighting.clone())),
                skylight: chunk.model.as_ref().map(|it| if it.needs_lighting_update { LightingBufferOrData::Data(chunk.skylight.clone()) } else { LightingBufferOrData::Buffer(it.skylight_buffer.clone()) }).unwrap_or(LightingBufferOrData::Data(chunk.skylight.clone())),
                cpx: self.chunks.get(ivec3(chunk_position[0]+1, chunk_position[1], chunk_position[2])).map(|it| it.data.blocks.clone()),
                cnx: self.chunks.get(ivec3(chunk_position[0]-1, chunk_position[1], chunk_position[2])).map(|it| it.data.blocks.clone()),
                cpy: self.chunks.get(ivec3(chunk_position[0], chunk_position[1]+1, chunk_position[2])).map(|it| it.data.blocks.clone()),
                cny: self.chunks.get(ivec3(chunk_position[0], chunk_position[1]-1, chunk_position[2])).map(|it| it.data.blocks.clone()),
                cpz: self.chunks.get(ivec3(chunk_position[0], chunk_position[1], chunk_position[2]+1)).map(|it| it.data.blocks.clone()),
                cnz: self.chunks.get(ivec3(chunk_position[0], chunk_position[1], chunk_position[2]-1)).map(|it| it.data.blocks.clone()),
            };
            let res = Self::generate_mesh_req(req, surface_ctx.device(), registries);
            self.chunks.get_mut(chunk_position).unwrap().model = res.chunk_model;
        }
        println!("took {:?} to generate model synchronously", SystemTime::now().duration_since(time).unwrap());
    }

    // pub async fn generate(&self, chunk_position: [i32; 3], surface_ctx: &dyn SurfaceCtx) {
    //     if self.chunks.get(chunk_position).await.is_none() {
    //         self.chunks.set(chunk_position, Chunk::new(surface_ctx)).await;
    //     }
    //     let chunk = self.chunks.get(chunk_position).await.unwrap();
    //     let output_buffer_bind_group = surface_ctx.device().create_bind_group(&BindGroupDescriptor {
    //         label: None,
    //         layout: &self.output_buffer_layout,
    //         entries: &[
    //             BindGroupEntry {
    //                 binding: 0,
    //                 resource: wgpu::BindingResource::Buffer(chunk.model.vertices.as_entire_buffer_binding())
    //             }
    //         ]
    //     });
    //     self.offset_uniform.lock().await.set_data(surface_ctx.queue(), [chunk_position[0] as f32 * CHUNK_SIZE as f32, chunk_position[1] as f32 *CHUNK_SIZE as f32, chunk_position[2] as f32 *CHUNK_SIZE as f32, 0.0]);
    //     self.shader.run_once(vec![&chunk.blocks.binding, &output_buffer_bind_group, &self.num_vertices_output.binding, &self.offset_uniform.lock().await.binding], [CHUNK_SIZE; 3], surface_ctx.device(), surface_ctx.queue());
    //     let num_vertices = *bytemuck::from_bytes::<u32>(&self.num_vertices_output.read(surface_ctx.device(), surface_ctx.queue()));
    //     let indices = (0..num_vertices/4).map(|i| [i*4, i*4+3, i*4+2, i*4, i*4+1, i*4+3]).flatten().collect::<Vec<u32>>();
    //     surface_ctx.queue().write_buffer(&chunk.model.indices, 0, bytemuck::cast_slice(&indices));
    //     *chunk.model.num_vertices.lock().await = num_vertices as usize;
    //     *chunk.model.num_indices.lock().await = indices.len();
    // }

    fn generate_blocks_req(req: GenerateChunkBlocksRequest, resources: &mut ChunkBlockGeneratorResources, callback: Option<impl FnMut(Vec<f64>)>) -> GenerateChunkBlocksResponse {
        let mut blocks = vec![0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize];
        if let Some(mut callback) = callback {
            let mut heights = vec![0.0; CHUNK_SIZE as usize * CHUNK_SIZE as usize];
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let height = resources.height_at(x as i32 +req.chunk_position[0]*CHUNK_SIZE as i32, z as i32 +req.chunk_position[2]*CHUNK_SIZE as i32, true).height;
                    heights[(x * CHUNK_SIZE + z) as usize] = height;
                }
            }
            callback(heights);
        }
        let mut max_cave = 0.0;
        let mut min_cave = 0.0;
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let HeightAt { height, gradient } = resources.height_at(x as i32 +req.chunk_position[0]*CHUNK_SIZE as i32, z as i32 +req.chunk_position[2]*CHUNK_SIZE as i32, true);
                for y in 0..CHUNK_SIZE {
                    let xf64 = x as f64 + req.chunk_position[0] as f64 * CHUNK_SIZE as f64;
                    let yf64 = y as f64+req.chunk_position[1] as f64 *(CHUNK_SIZE as f64);
                    let zf64 = z as f64 + req.chunk_position[2] as f64 * CHUNK_SIZE as f64;
                    let cave_position_scale = 20.0;
                    let cave_noise = (resources.cave_noise.get([xf64 / cave_position_scale, yf64 / cave_position_scale, zf64 / cave_position_scale ])+1.0)/2.0;
                    if cave_noise > max_cave {
                        max_cave = cave_noise
                    }
                    if cave_noise < min_cave {
                        min_cave = cave_noise
                    }
                    let cave_here = cave_noise > 0.90;
                    if yf64 <= height {
                        if yf64+5.0 < height || gradient.length() > 0.5 {
                            blocks[index_in_chunk(x, y, z)] = STONE;
                        } else if yf64+1.0 < height {
                            blocks[index_in_chunk(x, y, z)] = DIRT;
                        } else {
                            blocks[index_in_chunk(x, y, z)] = GRASS;
                        }
                    } else {
                        if yf64 <= 0.0 {
                            blocks[index_in_chunk(x, y, z)] = WATER;
                        } else {
                            blocks[index_in_chunk(x, y, z)] = AIR;
                        }
                    }
                    if cave_here && blocks[index_in_chunk(x, y, z)] != WATER {
                        blocks[index_in_chunk(x, y, z)] = AIR;
                    }
                }
            }
        }
        // println!("max: {max_cave}, min: {min_cave}");
        let mut features_placed = 0;
        fn place_feature(
            feature: impl Feature, attempts: usize, chance: f32, features_placed: &mut i32,
            resources: &mut ChunkBlockGeneratorResources, req: &GenerateChunkBlocksRequest,
            blocks: &mut Vec<u16>,
        ) {
            let chunk_sizei32 = CHUNK_SIZE as i32;
            let feature_type = feature.feature_type();
            let chunk_range = match feature_type {
                FeatureType::Surface => [-1..2, 0..1, -1..2],
                FeatureType::Carver => [-5..5, -5..5, -5..5],
                // FeatureType::Carver => [-10..10, -10..10, -10..10],
                FeatureType::Ore => [-1..2, -1..2, -1..2],
            };
            for cx in chunk_range[0].clone() {
                for cy in chunk_range[1].clone() {
                    for cz in chunk_range[2].clone() {
                        let mut hasher = DefaultHasher::new();
                        resources.seed.hash(&mut hasher);
                        features_placed.hash(&mut hasher);
                        let target_chunk_position = match feature_type {
                            FeatureType::Surface => [req.chunk_position[0]+cx, cy, req.chunk_position[2]+cz],
                            FeatureType::Carver | FeatureType::Ore => [req.chunk_position[0]+cx, req.chunk_position[1]+cy, req.chunk_position[2]+cz],
                        };
                        target_chunk_position.hash(&mut hasher);
                        let feature_seed = hasher.finish();
                        let mut rand = SmallRng::seed_from_u64(feature_seed);
                        for _ in 0..attempts {
                            if rand.random_range(0.0..1.0) > chance {
                                continue;
                            }
                            //same
                            let x = rand.random_range(0..chunk_sizei32)+cx*chunk_sizei32;
                            let z = rand.random_range(0..chunk_sizei32)+cz*chunk_sizei32;
                            let y = match feature_type {
                                FeatureType::Surface => {
                                    let in_this_chunk = x >= 0 && x < chunk_sizei32 && z >= 0 && z < chunk_sizei32;
                                    let height = resources.height_at(x+req.chunk_position[0]*chunk_sizei32, z+req.chunk_position[2]*chunk_sizei32, in_this_chunk).height;
                                    if height < 0.0 {
                                        continue;
                                    }
                                    height as i32 + 1 - (req.chunk_position[1])*chunk_sizei32
                                },
                                FeatureType::Ore | FeatureType::Carver => {
                                    let y = rand.random_range(0..chunk_sizei32)+cy*chunk_sizei32;
                                    let in_this_chunk = x >= 0 && x < chunk_sizei32 && z >= 0 && z < chunk_sizei32;
                                    let height = resources.height_at(x+req.chunk_position[0]*chunk_sizei32, z+req.chunk_position[2]*chunk_sizei32, in_this_chunk).height;
                                    if (y+req.chunk_position[1]*chunk_sizei32) as f64 > height {
                                        continue;
                                    }
                                    y
                                }
                            };
                            
                            feature.place(x+req.chunk_position.x*chunk_sizei32, y+req.chunk_position.y*chunk_sizei32, z+req.chunk_position.z*chunk_sizei32, &mut rand.fork(), |x: i32, y: i32, z: i32, block_id, condition| {
                                let x = x-req.chunk_position.x*chunk_sizei32;
                                let y = y-req.chunk_position.y*chunk_sizei32;
                                let z = z-req.chunk_position.z*chunk_sizei32;
                                if x >= 0 && x < chunk_sizei32 && y >= 0 && y < chunk_sizei32 && z >= 0 && z < chunk_sizei32 {
                                    let index = index_in_chunk(x as u32, y as u32, z as u32);
                                    if condition(blocks[index]) {
                                        blocks[index] = block_id;
                                    }
                                }
                            });
                        }
                    }
                }
            }
            features_placed.add_assign(1);
        }
        place_feature(WormCavesFeature {}, 4, 0.1, &mut features_placed, resources, &req, &mut blocks);
        // place_feature(AxisCavesFeature {}, 1, 0.001, &mut features_placed, resources, &req, &mut blocks);
        place_feature(OreFeature { block: blocks::IRON_ORE, num_blocks: 2..8 }, 100, 0.8, &mut features_placed, resources, &req, &mut blocks);
        place_feature(OreFeature { block: blocks::COPPER_ORE, num_blocks: 2..8 }, 100, 0.8, &mut features_placed, resources, &req, &mut blocks);
        place_feature(TreeFeature {}, 9, 0.7, &mut features_placed, resources, &req, &mut blocks);
        place_feature(BushFeature {}, 2, 0.9, &mut features_placed, resources, &req, &mut blocks);
        GenerateChunkBlocksResponse {
            chunk_position: req.chunk_position,
            chunk_blocks: blocks,
        }
    }

    fn generate_mesh_req(req: GenerateChunkMeshRequest, device: &Device, registries: &Registries) -> GenerateChunkModelResponse {
        // if req.chunk_blocks == [0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize] {
        //     return GenerateChunkModelResponse {
        //         chunk_model: None,
        //         chunk_position: req.chunk_position,
        //     };
        // }
        let lod = Self::lod_for_distance2(req.chunk_position, req.player_position);
        fn get_block(x: i32, y: i32, z: i32, req: &GenerateChunkMeshRequest, lod: i32) -> BlockID {
            let chunk_sizef32 = CHUNK_SIZE as f32;
            let chunk_sizei32 = CHUNK_SIZE as i32;
            if x >= 0 && y >= 0 && z >= 0 && x < chunk_sizei32 && y < chunk_sizei32 && z < chunk_sizei32 {
                if lod == 1 {
                    return req.chunk_blocks[index_in_chunk(x as u32, y as u32, z as u32)];
                } else {
                    let lx = x & !(lod - 1);
                    let ly = y & !(lod - 1);
                    let lz = z & !(lod - 1);
                    return req.chunk_blocks[index_in_chunk(lx as u32, ly as u32, lz as u32)];
                }
            } else {
                let cx = (x as f32 /chunk_sizef32).floor() as i32;
                let cy = (y as f32 /chunk_sizef32).floor() as i32;
                let cz = (z as f32 /chunk_sizef32).floor() as i32;
                let bx = x.rem_euclid(chunk_sizei32);
                let by = y.rem_euclid(chunk_sizei32);
                let bz = z.rem_euclid(chunk_sizei32);
                let blocks = match [cx, cy, cz] {
                    [1, 0, 0] => Some(&req.cpx),
                    [-1, 0, 0] => Some(&req.cnx),
                    [0, 1, 0] => Some(&req.cpy),
                    [0, -1, 0] => Some(&req.cny),
                    [0, 0, 1] => Some(&req.cpz),
                    [0, 0, -1] => Some(&req.cnz),
                    _ => None
                };
                if let Some(Some(blocks)) = blocks {
                    if lod == 1 {
                        return blocks[index_in_chunk(bx as u32, by as u32, bz as u32)];
                    } else {
                        let lx = bx & !(lod - 1);
                        let ly = by & !(lod - 1);
                        let lz = bz & !(lod - 1);
                        return blocks[index_in_chunk(lx as u32, ly as u32, lz as u32)];
                    }
                } else {
                    return AIR;
                }
            }
        }
        let chunk_position = req.chunk_position;
        // let mut vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        // let mut transparency_vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        let mut vertices_layers = vec![Vec::with_capacity(8192); 3];
        let mut modeled_vertices_layers = vec![Vec::with_capacity(256); 3];
        let mut modeled_indices_layers = vec![Vec::with_capacity(256); 3];
        let mut model_cache = HashMap::new();
        let air_block = registries.block_registry.get_block(&AIR);
        let mut mask = [air_block; (CHUNK_SIZE*CHUNK_SIZE) as usize];
        let mut block_cache: Vec<Option<Block>> = vec![];
        #[inline(always)]
        fn get_block_cached(block_id: BlockID, block_cache: &mut Vec<Option<Block>>, registries: &Registries) -> Block {
            if block_cache.len() > block_id as usize && let Some(block) = block_cache[block_id as usize] {
                return block;
            } else {
                let block = registries.block_registry.get_block(&block_id);
                if block_cache.len() <= block_id as usize {
                    block_cache.extend_from_slice(&vec![None; block_id as usize + 1 - block_cache.len()]);
                }
                block_cache[block_id as usize] = Some(block);
                block
            }
        }
        #[inline(always)]
        fn add_modeled_block(block: Block, model: &dyn BlockModelTrait, position: [i32; 3], chunk_position: IVec3, modeled_vertices: &mut Vec<Vertex>, modeled_indices: &mut Vec<u32>, model_cache: &mut HashMap<BlockID, BlockModel>) {
            let chunk_x_f32 = chunk_position[0] as f32 * CHUNK_SIZE as f32;
            let chunk_y_f32 = chunk_position[1] as f32 * CHUNK_SIZE as f32;
            let chunk_z_f32 = chunk_position[2] as f32 * CHUNK_SIZE as f32;
            let atlas_width_proportion = block.atlas_section.width as f32 / BLOCK_ATLAS_PNG_WIDTH as f32;
            let atlas_height_proportion = block.atlas_section.height as f32 / BLOCK_ATLAS_PNG_HEIGHT as f32; 
            modeled_vertices.extend(model.vertices().iter().map(|it| Vertex { position: [it.position[0]+position[0]as f32+chunk_x_f32, it.position[1]+position[1]as f32+chunk_y_f32, it.position[2]+position[2]as f32+chunk_z_f32, it.position[3]], color: [it.color[0], it.color[1], it.color[2], it.color[3]], normal: it.normal }));
            let num_indices = modeled_indices.len();
            modeled_indices.extend(model.indices().iter().map(|it| *it+num_indices as u32));
        }
        let chunk_sizei32 = CHUNK_SIZE as i32;
        for direction in [-1, 1] {
            for dim in 0..3 {
                let mut slice_direction = [0; 3];
                slice_direction[dim] = direction;
                let u = (dim+1)%3;
                let v = (dim+2)%3;
                let mut pos = [0; 3];
                for slice in 0..chunk_sizei32 {
                    let mut mask_i = 0;
                    for x in 0..chunk_sizei32 {
                        for y in 0..chunk_sizei32 {
                            pos[dim] = slice;
                            pos[u] = x;
                            pos[v] = y;
                            let block_here = get_block_cached(get_block(pos[0], pos[1], pos[2], &req, lod), &mut block_cache, registries);
                            if let Some(model) = block_here.model && lod == 1 {
                                add_modeled_block(block_here, model, pos, chunk_position, &mut modeled_vertices_layers[block_here.layer], &mut modeled_indices_layers[block_here.layer], &mut model_cache);
                                mask_i += 1;
                                continue;
                            }

                            let block_there = get_block_cached(get_block(pos[0]+slice_direction[0], pos[1]+slice_direction[1], pos[2]+slice_direction[2], &req, lod), &mut block_cache, registries);
                            mask[mask_i] = if ((block_here.id != block_there.id) && (block_here.layer < block_there.layer || block_there.model.is_some())) || !block_here.cull || !block_there.cull {
                                block_here
                            } else {
                                air_block
                            };
                            mask_i += 1;
                        }
                    }

                    // let mask_value = |x: i32, y: i32| -> i32 {
                    //     if x >= -1 && x <= CHUNK_SIZE as i32 && y >= -1 && y <= CHUNK_SIZE as i32 {
                    //         let index = (x+1)*(CHUNK_SIZE as i32 +2)+y+1;
                    //         return mask[index as usize];
                    //     } else {
                    //         return 0;
                    //     }
                    // };

                    #[inline(always)]
                    fn mask_index(x: i32, y: i32) -> usize {
                        (x*CHUNK_SIZE as i32+y) as usize
                    }
                    fn add_quad(
                        x: i32, y: i32, w: i32, h: i32, block: Block,
                        vertices_for: &mut Vec<Vertex>,
                        direction: i32, dim: usize, slice: i32,
                        chunk_position: IVec3,
                    ) {
                        // let num_vertices_for = match layer {
                        //     0 => &mut num_vertices,
                        //     1 => &mut num_transparency_vertices,
                        //     _ => panic!()
                        // };

                        let c = block.color;
                        let atlas_x = block.atlas_section.x;
                        let atlas_y = block.atlas_section.y;
                        let tex_width_x = block.atlas_section.width as f32 / BLOCK_ATLAS_PNG_WIDTH as f32;
                        let tex_width_y = block.atlas_section.height as f32 / BLOCK_ATLAS_PNG_HEIGHT as f32;
                        let tex_coords_offset_x = atlas_x as f32 / BLOCK_ATLAS_PNG_WIDTH as f32;
                        let tex_coords_offset_y = atlas_y as f32 / BLOCK_ATLAS_PNG_HEIGHT as f32;
                        let chunk_x_f32 = chunk_position[0] as f32 * CHUNK_SIZE as f32;
                        let chunk_y_f32 = chunk_position[1] as f32 * CHUNK_SIZE as f32;
                        let chunk_z_f32 = chunk_position[2] as f32 * CHUNK_SIZE as f32;

                        let direction_f32 = (direction as f32 + 1.0)/2.0;
                        if dim == 0 {
                            if direction == -1 {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + (x+w) as f32, chunk_z_f32 + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + (x+w) as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0],
                                    },
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + (x+w) as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + (x+w) as f32, chunk_z_f32 + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0],
                                    }
                                ]);
                            }
                        }
                        if dim == 1 {
                            if direction == -1 {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + (y+h) as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, -1.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, -1.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (y+h) as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]],
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, -1.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, -1.0, 0.0, 0.0],
                                    },
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (y+h) as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (y+h) as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]],
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0],
                                    }
                                ]);
                            }
                        }
                        if dim == 2 {
                            if direction == -1 {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0],
                                    },
                                    
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0],
                                    },//11
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0],
                                    },//01
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0],
                                    },//10
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0],
                                    }//00
                                ]);
                            }
                        }
                    }

                    let mut x = 0;
                    let mut y = 0;
                    while x < chunk_sizei32 {
                        while y < chunk_sizei32 {
                            let block = mask[mask_index(x, y)];
                            let mut h = 0;
                            let mut w = lod;
                            if block.layer != NOT_RENDERED_LAYER {
                                while y+h < chunk_sizei32 && mask[mask_index(x, y+h)] == block {
                                    h += lod;
                                }
                                'width: loop {
                                    if x+w >= chunk_sizei32 {
                                        break;
                                    }
                                    for i in (0..h).step_by(lod as usize) {
                                        if mask[mask_index(x+w, y+i)] != block {
                                            break 'width;
                                        }
                                    }
                                    w += lod;
                                }
                                add_quad(x, y, w, h, block, &mut vertices_layers[block.layer], direction, dim, slice, chunk_position);
                                for zero_x in x..x+w {
                                    for zero_y in y..y+h {
                                        mask[mask_index(zero_x, zero_y)] = air_block;
                                    }
                                }
                            }
                            y += lod;
                        }
                        if y >= chunk_sizei32 {
                            y = 0;
                            x += lod;
                        }
                    }
                }
            }
        }

        let max_num_indices = vertices_layers.iter().map(|it| it.len()).max().unwrap_or(0)/4;
        let mut indices = vec![0; max_num_indices*6];
        for i in 0..max_num_indices {
            indices[i*6+0] = i as u32 *4;
            indices[i*6+1] = i as u32 *4+3;
            indices[i*6+2] = i as u32 *4+2;
            indices[i*6+3] = i as u32 *4;
            indices[i*6+4] = i as u32 *4+1;
            indices[i*6+5] = i as u32 *4+3;
        }
        // let vertex_size = (vertices.len() * size_of::<Vertex>()) as u64;
        let index_size = (max_num_indices*6 * size_of::<u32>());

        // let t_vertex_size = (transparency_vertices.len() * size_of::<Vertex>()) as u64;

        // let m_vertex_size = (modeled_vertices.len() * size_of::<Vertex>()) as u64;
        // let m_index_size = (modeled_indices.len() * size_of::<u32>()) as u64;
        let all_vertices_size = vertices_layers.iter().map(|it| it.len()).sum::<usize>() * size_of::<Vertex>();
        let all_modeled_vertices_size = modeled_vertices_layers.iter().map(|it| it.len()).sum::<usize>() * size_of::<Vertex>();
        let all_modeled_indices_size = modeled_indices_layers.iter().map(|it| it.len()).sum::<usize>() * size_of::<u32>();
        let total_size = all_vertices_size+index_size+all_modeled_vertices_size+all_modeled_vertices_size;
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Chunk model"),
            mapped_at_creation: total_size > 0,
            size: total_size as u64,
            usage: BufferUsages::VERTEX | BufferUsages::INDEX,
        });
        if total_size > 0 {
            let mut mapped = buffer
                .get_mapped_range_mut(..).unwrap();
            mapped
                .slice(0..(index_size) as usize)
                .copy_from_slice(bytemuck::cast_slice(&indices));
            let mut start = index_size;
            for vertices in &vertices_layers {
                let vertex_size = vertices.len() * size_of::<Vertex>();
                mapped
                    .slice((start) as usize..(start+vertex_size) as usize)
                    .copy_from_slice(bytemuck::cast_slice(&vertices));
                start += vertex_size;
            }
            for i in 0..modeled_vertices_layers.len() {
                let modeled_vertices = &modeled_vertices_layers[i];
                let modeled_indices = &modeled_indices_layers[i];
                let m_vertices_size = modeled_vertices.len() * size_of::<Vertex>();
                let m_indices_size = modeled_indices.len() * size_of::<u32>();
                mapped
                    .slice(start as usize..(start+m_vertices_size) as usize)
                    .copy_from_slice(bytemuck::cast_slice(&modeled_vertices));
                start += m_vertices_size;
                mapped
                    .slice(start as usize..(start+m_indices_size) as usize)
                    .copy_from_slice(bytemuck::cast_slice(&modeled_indices));
                start += m_indices_size;
            }
            
            drop(mapped);
            buffer.unmap();
        }
        let layers = (0..3).map(|i| {
            ChunkModelLayer {
                num_vertices: vertices_layers[i].len(),
                num_indices: vertices_layers[i].len()/4*6,
                num_modeled_vertices: modeled_vertices_layers[i].len(),
                num_modeled_indices: modeled_indices_layers[i].len(),
            }
        }).collect();
        let lighting_buffer = match req.lighting {
            LightingBufferOrData::Buffer(buffer) => buffer,
            LightingBufferOrData::Data(lighting) => lighting.make_lighting_buffer(device),
        };
        let skylight_buffer = match req.skylight {
            LightingBufferOrData::Buffer(buffer) => buffer,
            LightingBufferOrData::Data(lighting) => lighting.make_lighting_buffer(device),
        };
        let chunk_model = Some(ChunkModel {
            buffer,
            layers,
            lighting_bind_group: device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: lighting_buffer.as_entire_binding(),
                }],
                label: None,
                layout: &Self::lighting_bind_group_layout(device),
            }),
            skylight_bind_group: device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: skylight_buffer.as_entire_binding(),
                }],
                label: None,
                layout: &Self::lighting_bind_group_layout(device),
            }),
            needs_lighting_update: false,
            lighting_buffer,
            skylight_buffer,
            chunk_position_uniform: UniformBinding::new(device, "Chunk position", req.chunk_position.as_vec3().into(), None),
        });
        
        
        GenerateChunkModelResponse {
            chunk_position,
            chunk_model,
            lod,
        }
    }

    pub fn lighting_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry { binding: 0, count: None, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT }],
            label: None,
        })
    }

    pub fn top_block_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry { binding: 0, count: None, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT }],
            label: None,
        })
    }

    pub fn get_chunk_or_create(&mut self, chunk_position: IVec3) -> &mut Chunk {
        let exists = self.chunks.get(chunk_position).is_some();
        if exists {
            return self.chunks.get_mut(chunk_position).unwrap();
        } else {
            self.chunks.set(chunk_position, Chunk::new(chunk_position));
            return self.chunks.get_mut(chunk_position).unwrap();
        }
    }

    pub fn replace_chunk(&mut self, chunk_position: IVec3, chunk: Chunk, registries: &Registries) {
        self.chunks.set(chunk_position, chunk);
        self.update_skylight_map_for_chunk(chunk_position, registries);
        self.update_lighting_around(chunk_position, registries);
    }

    pub fn update_skylight_map_for_chunk(&mut self, chunk_position: IVec3, registries: &Registries) {
        let pos_2d = ivec2(chunk_position.x, chunk_position.z);
        let mut skylight = if let Some(skylight) = self.skylights.skylights.remove(&pos_2d) {
            skylight
        } else {
            self.skylights.empty_top_block_data.clone()
        };

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if skylight.get_top_block(x, z) < chunk_position.y*CHUNK_SIZE as i32 {
                    'y: for y in (0..CHUNK_SIZE).rev() {
                        if registries.block_registry.get_block(&self.get_block(uvec3(x, y, z).as_ivec3()+chunk_position*CHUNK_SIZE as i32)).layer == SOLID_LAYER {
                            skylight.set_top_block(x, z, y as i32 + chunk_position.y*CHUNK_SIZE as i32);
                            break 'y;
                        }
                    }
                }
            }
        }
        self.skylights.skylights.insert(pos_2d, skylight);
    }

    pub fn update_skylight_map_for_added_block(&mut self, block_position: IVec3, block_id: BlockID, registries: &Registries, device: &Device) {
        if registries.block_registry.get_block(&block_id).layer != SOLID_LAYER {
            return;
        }
        let chunk_pos = chunk_for_block_position(block_position);
        let chunk_pos_2d = ivec2(chunk_pos.x, chunk_pos.z);
        let skylight = if let Some(skylight) = self.skylights.skylights.get_mut(&chunk_pos_2d) {
            skylight
        } else {
            self.skylights.skylights.insert(chunk_pos_2d, TopBlockData::new(device));
            self.skylights.skylights.get_mut(&chunk_pos_2d).unwrap()
        };

        let x = block_position.x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let z = block_position.z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if skylight.get_top_block(x, z) < block_position.y {
            skylight.set_top_block(x, z, block_position.y);
        }
    }

    pub fn update_skylight_map_for_removed_block(&mut self, block_position: IVec3, registries: &Registries, device: &Device) {
        let chunk_pos = chunk_for_block_position(block_position);
        let chunk_pos_2d = ivec2(chunk_pos.x, chunk_pos.z);
        let mut skylight = if let Some(skylight) = self.skylights.skylights.remove(&chunk_pos_2d) {
            skylight
        } else {
            TopBlockData::new(device)
        };

        let x = block_position.x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let z = block_position.z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if skylight.get_top_block(x, z) <= block_position.y {
        let mut y = block_position.y - 1;
            while let Some(block) = self.get_block_or_nah(ivec3(block_position.x, y, block_position.z)) {
                y -= 1;
                if block != AIR {
                    if registries.block_registry.get_block(&block).layer == SOLID_LAYER {
                        skylight.set_top_block(x, z, y);
                        break;
                    }
                }
            }
        }
        self.skylights.skylights.insert(chunk_pos_2d, skylight);
    }

    // fn chunk_distance(chunk_position: &[i32; 3], camera_position: [f32; 3]) -> f32 {
    //     Self::chunk_distance2(chunk_position, camera_position).sqrt()
    // }

    fn chunk_distance2_by_world(chunk_position: &IVec3, camera_position: [f32; 3]) -> f32 {
        let chunk_position_world = [(chunk_position[0] as f32+0.5) * CHUNK_SIZE as f32 , (chunk_position[1] as f32+0.5) * CHUNK_SIZE as f32, (chunk_position[2] as f32+0.5) * CHUNK_SIZE as f32];
        (chunk_position_world[0]-camera_position[0]).powi(2)+(chunk_position_world[1]-camera_position[1]).powi(2)+(chunk_position_world[2]-camera_position[2]).powi(2)
    }

    fn chunk_distance2_by_chunk(chunk_position: &IVec3, camera_position: [f32; 3]) -> i32 {
        let camera_position_chunk = [camera_position[0] as i32 / CHUNK_SIZE as i32, camera_position[1] as i32 / CHUNK_SIZE as i32, camera_position[2] as i32 / CHUNK_SIZE as i32];
        (camera_position_chunk[0]-chunk_position[0]).pow(2)+(camera_position_chunk[1]-chunk_position[1]).pow(2)+(camera_position_chunk[2]-chunk_position[2]).pow(2)
    }

    pub fn chunks_sorted(&self, camera_position: [f32; 3]) -> impl Iterator<Item = &Chunk> {
        self.chunks.main.keys().sorted_by_cached_key(|chunk_position_a| {
            OrderedFloat(Self::chunk_distance2_by_world(chunk_position_a, camera_position))
        }).map(|i| self.chunks.main.get(i).unwrap())
    }

    pub fn chunks(&self) -> impl Iterator<Item = (&IVec3, &Chunk)> {
        self.chunks.main.iter()
    }

    pub fn chunks_mut(&mut self) -> impl Iterator<Item = (&IVec3, &mut Chunk)> {
        self.chunks.main.iter_mut()
    }

    pub fn chunk_positions_owned(&self) -> Vec<IVec3> {
        self.chunks.main.keys().cloned().collect()
    }

    pub fn chunk_positions(&self) -> std::collections::hash_map::Keys<'_, IVec3, Chunk> {
        self.chunks.main.keys()
    }

    pub fn chunk_exists(&self, chunk_position: IVec3) -> bool {
        self.chunks.main.contains_key(&chunk_position)
    }

    pub fn chunk_loaded(&self, chunk_position: IVec3) -> bool {
        if self.chunks.main.contains_key(&chunk_position) {
            return self.chunks.main.get(&chunk_position).unwrap().data.generated_blocks;
        } else {
            return false;
        }
    }

    pub fn get_block(&self, world_position: IVec3) -> BlockID {
        self.get_block_or_nah(world_position).unwrap_or(AIR)
    }

    pub fn get_block_or_nah(&self, world_position: IVec3) -> Option<BlockID> {
        let x = world_position[0];
        let y = world_position[1];
        let z = world_position[2];
        let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
        let bx = x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let by = y.rem_euclid(CHUNK_SIZE as i32) as u32;
        let bz = z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if let Some(chunk) = self.chunks.get(ivec3(cx, cy, cz)) {
            return Some(chunk.data.blocks[index_in_chunk(bx, by, bz)]);
        } else {
            return None;
        }
    }

    pub fn get_block_state(&self, world_position: IVec3) -> Option<&Vec<u8>> {
        let x = world_position[0];
        let y = world_position[1];
        let z = world_position[2];
        let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
        let bx = x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let by = y.rem_euclid(CHUNK_SIZE as i32) as u32;
        let bz = z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if let Some(chunk) = self.chunks.get(ivec3(cx, cy, cz)) {
            return chunk.data.block_data.get(&uvec3(bx, by, bz));
        } else {
            return None;
        }
    }

    pub fn get_block_state_mut(&mut self, world_position: IVec3) -> Option<&mut Vec<u8>> {
        let x = world_position[0];
        let y = world_position[1];
        let z = world_position[2];
        let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
        let bx = x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let by = y.rem_euclid(CHUNK_SIZE as i32) as u32;
        let bz = z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if let Some(chunk) = self.chunks.get_mut(ivec3(cx, cy, cz)) {
            return chunk.data.block_data.get_mut(&uvec3(bx, by, bz));
        } else {
            return None;
        }
    }

    pub fn set_block(&mut self, world_position: IVec3, block_id: BlockID, state: Option<Vec<u8>>, update_synchronously: bool, registries: &Registries, device: &Device) {
        let x = world_position[0];
        let y = world_position[1];
        let z = world_position[2];
        let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
        let bx = x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let by = y.rem_euclid(CHUNK_SIZE as i32) as u32;
        let bz = z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if block_id == AIR {
            self.update_skylight_map_for_removed_block(world_position, registries, device);
        } else {
            self.update_skylight_map_for_added_block(world_position, block_id, registries, device);
        }
        if let Some(chunk) = self.chunks.main.get_mut(&ivec3(cx, cy, cz)) {
            chunk.set_block(uvec3(bx, by, bz), block_id, state, update_synchronously, registries);
        }
        self.update_lighting_around(ivec3(cx, cy, cz), registries);

    }

    pub fn update_lighting_around(&mut self, chunk_position: IVec3, registries: &Registries) {
        let mut lighting_grid = vec![BlockLightingData::new(); 3*3*3];
        let mut skylight_grid = vec![SkyLightingData::new(); 3*3*3];
        let mut set_or_get_lighting = |x, y, z, light| {
            let chunk_pos = chunk_for_block_position(ivec3(x, y, z));
            if chunk_pos.x > 1 || chunk_pos.y > 1 || chunk_pos.z > 1 || chunk_pos.x < -1 || chunk_pos.y < -1 || chunk_pos.z < -1 {
                return None;
            }
            let lighting = &mut lighting_grid[((chunk_pos.x+1) * 3*3 + (chunk_pos.y+1)*3 + chunk_pos.z + 1) as usize];
            let x = x.rem_euclid(CHUNK_SIZE as i32) as u32;
            let y = y.rem_euclid(CHUNK_SIZE as i32) as u32;
            let z = z.rem_euclid(CHUNK_SIZE as i32) as u32;
            if let Some(light) = light {
                lighting.set_color(x, y, z, light);
                return Some([0; 3]);
            } else {
                return Some(lighting.get_color(x, y, z));
            }
        };
        let mut set_or_get_skylight = |x, y, z, light| {
            let chunk_pos = chunk_for_block_position(ivec3(x, y, z));
            if chunk_pos.x > 1 || chunk_pos.y > 1 || chunk_pos.z > 1 || chunk_pos.x < -1 || chunk_pos.y < -1 || chunk_pos.z < -1 {
                return None;
            }
            let skylight = &mut skylight_grid[((chunk_pos.x+1) * 3*3 + (chunk_pos.y+1)*3 + chunk_pos.z + 1) as usize];
            let x = x.rem_euclid(CHUNK_SIZE as i32) as u32;
            let y = y.rem_euclid(CHUNK_SIZE as i32) as u32;
            let z = z.rem_euclid(CHUNK_SIZE as i32) as u32;
            if let Some(light) = light {
                skylight.set_color(x, y, z, light);
                return Some(0);
            } else {
                return Some(skylight.get_color(x, y, z));
            }
        };
        let mut block_cache = FxHashMap::default();
        let mut get_block = |block_id| {
            if block_cache.contains_key(&block_id) {
                block_cache.get(&block_id).cloned().unwrap()
            } else {
                let block = registries.block_registry.get_block(&block_id);
                &block_cache.insert(block.id, block);
                block
            }
        };
        let mut updates = VecDeque::new();
        let mut skylight_updates = VecDeque::new();
        for x in -(CHUNK_SIZE as i32)+1..CHUNK_SIZE as i32*2 {
            for z in -(CHUNK_SIZE as i32)+1..CHUNK_SIZE as i32*2 {
                let top_block_here = self.top_block(x+chunk_position.x * CHUNK_SIZE as i32, z+chunk_position.z * CHUNK_SIZE as i32);
                for y in -(CHUNK_SIZE as i32)+1..CHUNK_SIZE as i32*2 {
                    let skylight_here = top_block_here <= y as i32 +chunk_position.y * CHUNK_SIZE as i32;
                    let skylight_here = if skylight_here { 15 } else { 0 };
                    let here_relative_pos = ivec3(x, y as i32, z);
                    if let Some(chunk) = self.chunks.get(chunk_for_block_position(here_relative_pos+chunk_position * CHUNK_SIZE as i32)) {
                        let x = x.rem_euclid(CHUNK_SIZE as i32) as u32;
                        let y = y.rem_euclid(CHUNK_SIZE as i32) as u32;
                        let z = z.rem_euclid(CHUNK_SIZE as i32) as u32;
                        let lighting_emission = get_block(chunk.data.blocks[index_in_chunk(x, y, z)]).lighting_emission;
                        if lighting_emission != [0; 3] {
                            set_or_get_lighting(here_relative_pos.x, here_relative_pos.y, here_relative_pos.z, Some(lighting_emission));
                            updates.push_back(here_relative_pos);
                        }
                        if skylight_here != 0 {
                            set_or_get_skylight(here_relative_pos.x, here_relative_pos.y, here_relative_pos.z, Some(skylight_here));
                            skylight_updates.push_back(here_relative_pos);
                        }
                    }
                }
            }
        }
        while let Some(update_relative_pos) = updates.pop_front() {
            let light_here = set_or_get_lighting(update_relative_pos.x, update_relative_pos.y, update_relative_pos.z, None).unwrap();
            for relative_neighbor in neighbors(update_relative_pos) {
                if let Some(chunk) = self.chunks.get(chunk_for_block_position(relative_neighbor+chunk_position * CHUNK_SIZE as i32)) {
                    let x = relative_neighbor.x.rem_euclid(CHUNK_SIZE as i32) as u32;
                    let y = relative_neighbor.y.rem_euclid(CHUNK_SIZE as i32) as u32;
                    let z = relative_neighbor.z.rem_euclid(CHUNK_SIZE as i32) as u32;
                    if let Some(light_there) = set_or_get_lighting(relative_neighbor.x, relative_neighbor.y, relative_neighbor.z, None) {
                        let less_light = [light_here[0].checked_sub(1).unwrap_or(0).max(light_there[0]), light_here[1].checked_sub(1).unwrap_or(0).max(light_there[1]), light_here[2].checked_sub(1).unwrap_or(0).max(light_there[2])];
                        if less_light != light_there {
                            set_or_get_lighting(relative_neighbor.x, relative_neighbor.y, relative_neighbor.z, Some(less_light));
                            if get_block(chunk.data.blocks[index_in_chunk(x, y, z)]).layer != SOLID_LAYER {
                                updates.push_back(relative_neighbor);
                            }
                        }
                    }
                }
            }
        }
        while let Some(update_relative_pos) = skylight_updates.pop_front() {
            let light_here = set_or_get_skylight(update_relative_pos.x, update_relative_pos.y, update_relative_pos.z, None).unwrap();
            for relative_neighbor in neighbors(update_relative_pos) {
                if let Some(chunk) = self.chunks.get(chunk_for_block_position(relative_neighbor+chunk_position * CHUNK_SIZE as i32)) {
                    let x = relative_neighbor.x.rem_euclid(CHUNK_SIZE as i32) as u32;
                    let y = relative_neighbor.y.rem_euclid(CHUNK_SIZE as i32) as u32;
                    let z = relative_neighbor.z.rem_euclid(CHUNK_SIZE as i32) as u32;
                    if let Some(light_there) = set_or_get_skylight(relative_neighbor.x, relative_neighbor.y, relative_neighbor.z, None) {
                        let less_light = light_here.checked_sub(1).unwrap_or(0).max(light_there);
                        if less_light != light_there {
                            set_or_get_skylight(relative_neighbor.x, relative_neighbor.y, relative_neighbor.z, Some(less_light));
                            if get_block(chunk.data.blocks[index_in_chunk(x, y, z)]).layer != SOLID_LAYER {
                                skylight_updates.push_back(relative_neighbor);
                            }
                        }
                    }
                }
            }
        }
        for x in (-1..=1).rev() {
            for y in (-1..=1).rev() {
                for z in (-1..=1).rev() {
                    let lighting = lighting_grid.pop().unwrap();
                    let skylight = skylight_grid.pop().unwrap();
                    if let Some(chunk) = self.chunks.get_mut(chunk_position+ivec3(x, y, z)) { 
                        chunk.lighting = lighting;
                        chunk.skylight = skylight;
                        if let Some(model) = &mut chunk.model {
                            model.needs_lighting_update = true;
                        }
                    }
                }
            }
        }
    }

    pub fn lod_for_distance2(chunk_position: IVec3, player_position: [f32; 3]) -> i32 {
        2i32.pow((chunk_position.as_vec3().mul(CHUNK_SIZE as f32).distance_squared(Vec3::from(player_position))/250.0f32.powi(2).floor()) as u32).min(4).max(1)
    }

    // pub fn top_block_bind_group(&self, chunk_pos: IVec3) -> &TopBlockData {
    //     if let Some(top_block_data) = self.skylights.skylights.get(&ivec2(chunk_pos.x, chunk_pos.z)) {
    //         top_block_data
    //     } else {
    //         return &self.skylights.empty_top_block_data
    //     }
    // }

    pub fn top_block(&self, x: i32, z: i32) -> i32 {
        let chunk_pos = chunk_for_block_position(ivec3(x, 0, z));
        let x = x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let z = z.rem_euclid(CHUNK_SIZE as i32) as u32;
        self.skylights.skylights.get(&ivec2(chunk_pos.x, chunk_pos.z)).map(|it| it.get_top_block(x, z)).unwrap_or(0)
    }

    // pub fn clean_top_block_data(&mut self, chunk_pos: IVec2, device: &Device) {
    //     if let Some(top_block_data) = self.skylights.skylights.get_mut(&chunk_pos) {
    //         top_block_data.update_buffer(device);
    //     }
    // }
}

struct ChunkMap {
    pub main: FxHashMap<IVec3, Chunk>,
}

impl ChunkMap {
    fn new() -> Self {
        Self {
            main: FxHashMap::default(),
        }
    }

    fn get(&self, key: IVec3) -> Option<&Chunk> {
        self.main.get(&key)
    }

    fn get_mut(&mut self, key: IVec3) -> Option<&mut Chunk> {
        self.main.get_mut(&key)
    }

    fn set(&mut self, key: IVec3, chunk: Chunk) -> Option<Chunk> {
        self.main.insert(key, chunk)
    }
}

struct SkylightMap {
    skylights: FxHashMap<IVec2, TopBlockData>,
    empty_top_block_data: TopBlockData,
}

impl SkylightMap {
    pub fn new(device: &Device) -> Self {
        Self {
            skylights: FxHashMap::default(),
            empty_top_block_data: TopBlockData::new(device),
        }
    }
}

pub type NoiseType = Perlin;

struct ChunkBlockGeneratorResources {
    seed: u32,
    terrain_noises: Vec<NoiseType>,
    cave_noise: NoiseType,
    cached_noise_map: Option<([i32; 2], Vec<f64>)>,
}

struct HeightAt {
    height: f64,
    gradient: DVec2,
}

impl ChunkBlockGeneratorResources {
    fn new(seed: u32) -> Self {
        let terrain_noises = (0..7).map(|i| NoiseType::new(seed+i)).collect::<Vec<_>>();
        let cave_noise = NoiseType::new(seed);
        Self {
            seed,
            terrain_noises,
            cave_noise,
            cached_noise_map: None,
        }
    }

    fn height_at(&mut self, mut x: i32, mut z: i32, cache: bool) -> HeightAt {
        let mut temp_noise_map;
        let map_width;
        let map_height;
        let gen_width;
        let gen_height;
        let noise_map;
        let chunk_pos: IVec3 = chunk_for_block_position(ivec3(x, 0, z)).into();
        let base_xf64 = x as f64;
        let base_zf64 = z as f64;
        if cache {
            let chunk_size1 = CHUNK_SIZE as usize + 1;
            if self.cached_noise_map.as_ref().is_none_or(|it| it.0 != [chunk_pos.x, chunk_pos.z]) {
                let noise_map = vec![0.0; (chunk_size1*chunk_size1)*(self.terrain_noises.len()+1)];
                self.cached_noise_map = Some(([chunk_pos.x, chunk_pos.z], noise_map));
                gen_width = chunk_size1;
                gen_height = chunk_size1;
            } else {
                gen_width = 0;
                gen_height = 0;
            }
            map_width = chunk_size1;
            map_height = chunk_size1;
            noise_map = &mut self.cached_noise_map.as_mut().unwrap().1;
            x = x - chunk_pos.x*CHUNK_SIZE as i32;
            z = z - chunk_pos.z*CHUNK_SIZE as i32;
        } else {
            temp_noise_map = vec![0.0; (2*2)*(self.terrain_noises.len()+1)];
            map_width = 2;
            map_height = 2;
            gen_width = 2;
            gen_height = 2;
            noise_map = &mut temp_noise_map;
            x = 0;
            z = 0;
        }

        for x in 0..gen_width {
            for z in 0..gen_height {
                let mut position_scale = 1000.0;
                let mut height_scale = 100.0;
                let mut height = 0.0;
                let xf64 = base_xf64 + x as f64;
                let zf64 = base_zf64 + z as f64;
                let mut noise_gradient = dvec2(0.0, 0.0);
                for (noise_i, noise) in self.terrain_noises.iter().enumerate() {
                    position_scale *= 0.5;
                    height_scale *= 0.5;
                    let noise_here = if x == 0 && z == 0 {
                        let noise = noise.get([xf64 / position_scale, zf64 / position_scale]);
                        noise_map[noise_i] = noise;
                        noise
                    } else {
                        noise_map[x as usize * map_height * (self.terrain_noises.len()+1) + z as usize * (self.terrain_noises.len()+1) + noise_i]
                    };
                    let noise_px = noise.get([(xf64+1.0) / position_scale, (zf64) / position_scale]);
                    if x+1 < map_width {
                        noise_map[(x+1) as usize * map_height * (self.terrain_noises.len()+1) + z as usize * (self.terrain_noises.len()+1) + noise_i] = noise_px;
                    }
                    let noise_pz = noise.get([(xf64) / position_scale, (zf64+1.0) / position_scale]);
                    if z+1 < map_height {
                        noise_map[x as usize * map_height * (self.terrain_noises.len()+1) + (z+1) as usize * (self.terrain_noises.len()+1) + noise_i] = noise_pz;
                    }
                    noise_gradient += dvec2(noise_here-noise_px, noise_here-noise_pz)/position_scale;
                    height += noise_here * height_scale * 1.0/(1.0+noise_gradient.length());
                }
                noise_map[x as usize * map_height * (self.terrain_noises.len()+1) + z as usize * (self.terrain_noises.len()+1) + self.terrain_noises.len()] = height;
            }
        }
        let height = noise_map[x as usize * map_height * (self.terrain_noises.len()+1) + z as usize * (self.terrain_noises.len()+1) + self.terrain_noises.len()];
        let height_px = noise_map[(x as usize+1) * map_height * (self.terrain_noises.len()+1) + z as usize * (self.terrain_noises.len()+1) + self.terrain_noises.len()];
        let height_pz = noise_map[x as usize * map_height * (self.terrain_noises.len()+1) + (z as usize+1) * (self.terrain_noises.len()+1) + self.terrain_noises.len()];
        let gradient = dvec2(height_px-height, height_pz-height);
        return HeightAt { height, gradient }
    }
}

#[cfg(test)]
mod worldgen_test {
    use std::{fs::write, io::Cursor, path::Path};

use glam::ivec3;
use image::{ImageBuffer, ImageFormat, Luma};

use crate::chunk::{CHUNK_SIZE, ChunkManager};

    #[test]
    fn test_worldgen() {
        let mut image = ImageBuffer::<Luma<u16>, Vec<u16>>::new(CHUNK_SIZE*100, CHUNK_SIZE*100);
        let mut resources = super::ChunkBlockGeneratorResources::new(rand::random());
        for cx in 0..100 {
            for cz in 0..100 {
                println!("{}/10,000", cx*100+cz);
                ChunkManager::generate_blocks_req(super::GenerateChunkBlocksRequest { chunk_position: ivec3(cx as i32, 0, cz as i32), player_position: [(cx * CHUNK_SIZE) as f32, 0.0, (cz * CHUNK_SIZE) as f32] }, &mut resources, Some(|heights: Vec<f64>| {
                    for x in 0..CHUNK_SIZE {
                        for z in 0..CHUNK_SIZE {
                            let height = heights[(x * CHUNK_SIZE + z) as usize];
                            image.put_pixel(x+cx*CHUNK_SIZE, z+cz*CHUNK_SIZE, Luma([(u16::MAX as f64 * ((height+500.0)/1000.0)) as u16]));
                        }
                    }
                }));
            }
        }
        let mut output = vec![];
        image.write_to(&mut Cursor::new(&mut output), ImageFormat::Png).unwrap();
        write(&Path::new("worldgen_heights.png"), output).unwrap();
    }
}