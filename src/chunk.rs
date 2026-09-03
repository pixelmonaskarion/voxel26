use std::{collections::HashMap, hash::{DefaultHasher, Hash, Hasher}, ops::{AddAssign, Mul}, sync::mpsc, time::{Duration, SystemTime}};

use bespoke_engine::{binding::UniformBinding, model::Render, shader::Shader, surface_context::SurfaceCtx, texture::Texture};
use cgmath::{InnerSpace, MetricSpace, Vector3, vec2, vec3};
use noise::{NoiseFn, Perlin};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, Device, RenderPass};
use itertools::Itertools;

use crate::{BLOCK_ATLAS_PNG_HEIGHT, BLOCK_ATLAS_PNG_WIDTH, block_models::{BlockModel, parse_model}, blocks::{self, AIR, Block, BlockID, DIRT, GOLD, GRASS, NOT_RENDERED_LAYER, STONE, WATER}, entity::{Entity, EntityRenderManager, EntityType}, features::{Feature, bush::BushFeature, tree::TreeFeature}, game::Vertex, inventory::ItemAtlas, util::neighbors};

pub struct Chunk {
    blocks: Vec<BlockID>,
    generated_blocks: bool,
    model: Option<ChunkModel>,
    pub entities: HashMap<EntityType, Vec<Entity>>,
    pub needed_chunk_updates: Vec<NeededChunkUpdate>,
    // transparency_model: Option<ChunkModel>,
}

pub struct ChunkModel {
    // vertices: Buffer,
    buffer: Buffer,
    num_vertices: usize,
    num_indices: usize,
    num_transparent_vertices: usize,
    num_transparent_indices: usize,
    num_modeled_vertices: usize,
    num_modeled_indices: usize,
}

pub struct NeededChunkUpdate {
    pub relative_chunk_pos: Vector3<i32>,
    pub synchronous: bool,
}

pub const CHUNK_SIZE: u32 = 32;

#[inline(always)]
pub fn index_in_chunk(x: u32, y: u32, z: u32) -> usize {
    (y * CHUNK_SIZE * CHUNK_SIZE + x * CHUNK_SIZE + z) as usize
}

#[allow(unused)]
impl Chunk {
    pub fn new() -> Self {
        let mut _self = Self {
            blocks: vec![0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize],
            generated_blocks: false,
            model: None,
            entities: HashMap::new(),
            needed_chunk_updates: vec![],
        };
        _self
    }

    pub fn set_block(&mut self, local_coords: [u32; 3], block: BlockID, update_synchronously: bool) {
        self.blocks[index_in_chunk(local_coords[0], local_coords[1], local_coords[2])] = block;
        self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: vec3(0, 0, 0), synchronous: update_synchronously });
        if local_coords[0] == 0 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: vec3(-1, 0, 0), synchronous: update_synchronously });
        }
        if local_coords[1] == 0 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: vec3(0, -1, 0), synchronous: update_synchronously });
        }
        if local_coords[2] == 0 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: vec3(0, 0, -1), synchronous: update_synchronously });
        }
        if local_coords[0] == CHUNK_SIZE-1 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: vec3(1, 0, 0), synchronous: update_synchronously });
        }
        if local_coords[1] == CHUNK_SIZE-1 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: vec3(0, 1, 0), synchronous: update_synchronously });
        }
        if local_coords[2] == CHUNK_SIZE-1 {
            self.needed_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: vec3(0, 0, 1), synchronous: update_synchronously });
        }
    }

    pub fn render<'s: 'b, 'b>(&'s self, render_pass: &mut RenderPass<'b>, entity_render_manager: &'b EntityRenderManager, item_atlas: &ItemAtlas, chunk_shader: &'b Shader<'b>, atlas_binding: &UniformBinding<Texture>, surface_ctx: &dyn SurfaceCtx) {
        chunk_shader.bind(render_pass);
        render_pass.set_bind_group(1, &atlas_binding.binding, &[]);
        if let Some(model) = &self.model {
            if model.num_vertices > 0 {
                render_pass.set_vertex_buffer(0, model.buffer.slice(..(model.num_vertices*size_of::<Vertex>()) as u64));
                render_pass.set_index_buffer(model.buffer.slice(((model.num_vertices+model.num_transparent_vertices)*size_of::<Vertex>()) as u64..((model.num_vertices+model.num_transparent_vertices)*size_of::<Vertex>()+model.num_indices*size_of::<u32>()) as u64), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model.num_indices as u32, 0, 0..1);
            }
            if model.num_modeled_vertices > 0 {
                render_pass.set_vertex_buffer(0, model.buffer.slice(((model.num_vertices+model.num_transparent_vertices)*size_of::<Vertex>()+model.num_indices*size_of::<u32>()) as u64..((model.num_vertices+model.num_transparent_vertices+model.num_modeled_vertices)*size_of::<Vertex>()+model.num_indices.max(model.num_transparent_indices)*size_of::<u32>()) as u64));
                render_pass.set_index_buffer(model.buffer.slice(((model.num_vertices+model.num_transparent_vertices+model.num_modeled_vertices)*size_of::<Vertex>()+model.num_indices*size_of::<u32>()) as u64..((model.num_vertices+model.num_transparent_vertices+model.num_modeled_vertices)*size_of::<Vertex>()+(model.num_indices.max(model.num_transparent_indices)+model.num_modeled_indices)*size_of::<u32>()) as u64), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model.num_modeled_indices as u32, 0, 0..1);
            }
            for (entity_type, entities) in &self.entities {
                if entities.len() == 0 {
                    continue;
                }
                match *entity_type {
                    EntityType::Item => {
                        entity_render_manager.item_shader.bind(render_pass);
                        render_pass.set_bind_group(1, &item_atlas.texture.binding, &[]);
                    }
                    _ => {}
                }
                let max_batch_size = entity_render_manager.instance_buffer.size() as usize;
                let mut instance_size = 0;
                let instances = entities.iter().flat_map(|it| {
                    let instance = it.shader_instance(item_atlas);
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
            chunk_shader.bind(render_pass);
            render_pass.set_bind_group(1, &atlas_binding.binding, &[]);
            if model.num_transparent_vertices > 0 {
                render_pass.set_vertex_buffer(0, model.buffer.slice((model.num_vertices*size_of::<Vertex>()) as u64..(model.num_vertices*size_of::<Vertex>()+model.num_transparent_vertices*size_of::<Vertex>()) as u64));
                render_pass.set_index_buffer(model.buffer.slice(((model.num_vertices+model.num_transparent_vertices)*size_of::<Vertex>()) as u64..((model.num_vertices+model.num_transparent_vertices)*size_of::<Vertex>()+model.num_transparent_indices*size_of::<u32>()) as u64), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model.num_transparent_indices as u32, 0, 0..1);
            }
        }
    }

    pub fn visible(&self) -> bool {
        self.model.as_ref().is_some_and(|it| it.num_vertices > 0 || it.num_transparent_vertices > 0)
    }

    pub fn add_entity(&mut self, entity: Entity) {
        let id = entity.id();
        if !self.entities.contains_key(&id) {
            self.entities.insert(id, vec![]);
        }
        self.entities.get_mut(&id).unwrap().push(entity);
    }
}

struct GenerateChunkBlocksRequest {
    chunk_position: [i32; 3],
    player_position: [f32; 3],
}

struct GenerateChunkBlocksResponse {
    chunk_position: [i32; 3],
    chunk_blocks: Vec<BlockID>,
}

struct GenerateChunkMeshRequest {
    chunk_position: [i32; 3],
    player_position: [f32; 3],
    chunk_blocks: Vec<BlockID>,
    cpx: Option<Vec<BlockID>>,
    cnx: Option<Vec<BlockID>>,
    cpy: Option<Vec<BlockID>>,
    cny: Option<Vec<BlockID>>,
    cpz: Option<Vec<BlockID>>,
    cnz: Option<Vec<BlockID>>,
}

struct GenerateChunkModelResponse {
    chunk_position: [i32; 3],
    chunk_model: Option<ChunkModel>,
}

pub struct ChunkManager {
    chunks: ChunkMap,
    gen_blocks_tx: mpsc::Sender<GenerateChunkBlocksRequest>,
    gen_blocks_rx: mpsc::Receiver<GenerateChunkBlocksResponse>,
    gen_model_tx: mpsc::Sender<GenerateChunkMeshRequest>,
    gen_model_rx: mpsc::Receiver<GenerateChunkModelResponse>,
}
#[allow(unused)]
impl ChunkManager {
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let (gen_blocks_req_tx, gen_blocks_req_rx) = mpsc::channel::<GenerateChunkBlocksRequest>();
        let (gen_blocks_res_tx, gen_blocks_res_rx) = mpsc::channel();
        let (gen_model_req_tx, gen_model_req_rx) = mpsc::channel::<GenerateChunkMeshRequest>();
        let (gen_model_res_tx, gen_model_res_rx) = mpsc::channel();
        let device = surface_ctx.device_arc().clone();
        let seed = rand::random();
        tokio::spawn(async move {
            let mut n = 0;
            let mut t = Duration::ZERO;
            let mut queue: Vec<GenerateChunkBlocksRequest> = Vec::new();
            let mut player_position: [f32; 3];
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
                    gen_blocks_res_tx.send(Self::generate_blocks_req(req, seed)).unwrap();
                    t += SystemTime::now().duration_since(start).unwrap();
                    n += 1;
                    if n % 1000 == 999 {
                        // let avg = t/n;
                        // println!("avg gen blocks: {avg:?}");
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
                    gen_model_res_tx.send(Self::generate_mesh_req(req, &device)).unwrap();
                    t += SystemTime::now().duration_since(start).unwrap();
                    n += 1;
                    if n % 1000 == 999 {
                        // let avg = t/n;
                        // println!("avg gen mesh: {avg:?}");
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
            gen_blocks_tx: gen_blocks_req_tx,
            gen_blocks_rx: gen_blocks_res_rx,
            gen_model_tx: gen_model_req_tx,
            gen_model_rx: gen_model_res_rx,
        }
    }

    pub fn poll_channels(&mut self, player_position: [f32; 3]) {
        while let Ok(res) = self.gen_blocks_rx.try_recv() {
            if let Some(chunk) = self.chunks.get_mut(res.chunk_position) {
                chunk.blocks = res.chunk_blocks;
                chunk.generated_blocks = true;
                self.generate_model(res.chunk_position, player_position);
                for chunk_position in neighbors(res.chunk_position) {
                    self.generate_model(chunk_position, player_position);
                }
            }
        }
        while let Ok(res) = self.gen_model_rx.try_recv() {
            if let Some(chunk) = self.chunks.get_mut(res.chunk_position) {
                chunk.model = res.chunk_model;
                // chunk.transparency_model = res.transparency_model;
            }
        }
    }

    pub fn generate_blocks(&self, chunk_position: [i32; 3], player_position: [f32; 3]) {
        self.gen_blocks_tx.send(GenerateChunkBlocksRequest {
            chunk_position,
            player_position,
        }).unwrap();
    }

    pub fn generate_model(&self, chunk_position: [i32; 3], player_position: [f32; 3]) {
        if let Some(chunk_blocks) = self.chunks.get(chunk_position).map(|it| it.blocks.clone()) {
            self.gen_model_tx.send(GenerateChunkMeshRequest {
                chunk_position,
                player_position,
                chunk_blocks,
                cpx: self.chunks.get([chunk_position[0]+1, chunk_position[1], chunk_position[2]]).map(|it| it.blocks.clone()),
                cnx: self.chunks.get([chunk_position[0]-1, chunk_position[1], chunk_position[2]]).map(|it| it.blocks.clone()),
                cpy: self.chunks.get([chunk_position[0], chunk_position[1]+1, chunk_position[2]]).map(|it| it.blocks.clone()),
                cny: self.chunks.get([chunk_position[0], chunk_position[1]-1, chunk_position[2]]).map(|it| it.blocks.clone()),
                cpz: self.chunks.get([chunk_position[0], chunk_position[1], chunk_position[2]+1]).map(|it| it.blocks.clone()),
                cnz: self.chunks.get([chunk_position[0], chunk_position[1], chunk_position[2]-1]).map(|it| it.blocks.clone()),
            }).unwrap();
        }
    }

    pub fn generate_model_and_surroundings(&self, chunk_position: [i32; 3], player_position: [f32; 3]) {
        self.generate_model(chunk_position, player_position);
        for position in neighbors(chunk_position) {
            self.generate_model(position, player_position);
        }
    }

    pub fn generate_model_and_surroundings_now(&mut self, chunk_position: [i32; 3], player_position: [f32; 3], surface_ctx: &dyn SurfaceCtx) {
        self.generate_model_now(chunk_position, player_position, surface_ctx);
        for position in neighbors(chunk_position) {
            self.generate_model_now(position, player_position, surface_ctx);
        }
    }

    pub fn generate_model_now(&mut self, chunk_position: [i32; 3], player_position: [f32; 3], surface_ctx: &dyn SurfaceCtx) {
        let time = SystemTime::now();
        if let Some(chunk_blocks) = self.chunks.get(chunk_position).map(|it| it.blocks.clone()) {
            let req = GenerateChunkMeshRequest {
                chunk_position,
                player_position,
                chunk_blocks,
                cpx: self.chunks.get([chunk_position[0]+1, chunk_position[1], chunk_position[2]]).map(|it| it.blocks.clone()),
                cnx: self.chunks.get([chunk_position[0]-1, chunk_position[1], chunk_position[2]]).map(|it| it.blocks.clone()),
                cpy: self.chunks.get([chunk_position[0], chunk_position[1]+1, chunk_position[2]]).map(|it| it.blocks.clone()),
                cny: self.chunks.get([chunk_position[0], chunk_position[1]-1, chunk_position[2]]).map(|it| it.blocks.clone()),
                cpz: self.chunks.get([chunk_position[0], chunk_position[1], chunk_position[2]+1]).map(|it| it.blocks.clone()),
                cnz: self.chunks.get([chunk_position[0], chunk_position[1], chunk_position[2]-1]).map(|it| it.blocks.clone()),
            };
            let res = Self::generate_mesh_req(req, surface_ctx.device());
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

    fn generate_blocks_req(req: GenerateChunkBlocksRequest, seed: u32) -> GenerateChunkBlocksResponse {
        let mut blocks = vec![0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize];
        let noises = (0..8).map(|i| Perlin::new(seed+i)).collect::<Vec<_>>();
        let chunk_size1 = CHUNK_SIZE as usize + 1;
        let mut noise_map = vec![0.0; (chunk_size1*chunk_size1)*(noises.len()+1)];
        for x in 0..chunk_size1 {
            for z in 0..chunk_size1 {
                let mut position_scale = 2000.0;
                let mut height_scale = 300.0;
                let mut height = 0.0;
                let xf64 = x as f64 + req.chunk_position[0] as f64 * CHUNK_SIZE as f64;
                let zf64 = z as f64 + req.chunk_position[2] as f64 * CHUNK_SIZE as f64;
                let mut noise_gradient = vec2(0.0, 0.0);
                for (noise_i, noise) in noises.iter().enumerate() {
                    position_scale *= 0.6;
                    height_scale *= 0.5;
                    let noise_here = if x == 0 && z == 0 {
                        let noise = noise.get([xf64 / position_scale, zf64 / position_scale]);
                        noise_map[noise_i] = noise;
                        noise
                    } else {
                        noise_map[x as usize * chunk_size1 * (noises.len()+1) + z as usize * (noises.len()+1) + noise_i]
                    };
                    let noise_px = noise.get([(xf64+1.0) / position_scale, (zf64) / position_scale]);
                    if x != CHUNK_SIZE as usize {
                        noise_map[(x+1) as usize * chunk_size1 * (noises.len()+1) + z as usize * (noises.len()+1) + noise_i] = noise_px;
                    }
                    let noise_pz = noise.get([(xf64) / position_scale, (zf64+1.0) / position_scale]);
                    if z != CHUNK_SIZE as usize {
                        noise_map[x as usize * chunk_size1 * (noises.len()+1) + (z+1) as usize * (noises.len()+1) + noise_i] = noise_pz;
                    }
                    noise_gradient += vec2(noise_here-noise_px, noise_here-noise_pz)/position_scale;
                    height += noise_here * height_scale * 1.0/(1.0+noise_gradient.magnitude());
                }
                noise_map[x as usize * chunk_size1 * (noises.len()+1) + z as usize * (noises.len()+1) + noises.len()] = height;
            }
        }
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let height = noise_map[x as usize * chunk_size1 * (noises.len()+1) + z as usize * (noises.len()+1) + noises.len()];
                let height_px = noise_map[(x+1) as usize * chunk_size1 * (noises.len()+1) + z as usize * (noises.len()+1) + noises.len()];
                let height_pz = noise_map[x as usize * chunk_size1 * (noises.len()+1) + (z+1) as usize * (noises.len()+1) + noises.len()];
                let height_gradient = vec2(height_px-height, height_pz-height);
                for y in 0..CHUNK_SIZE {
                    let yf64 = y as f64+req.chunk_position[1] as f64 *(CHUNK_SIZE as f64);
                    if yf64 <= height {
                        if yf64+5.0 < height || height_gradient.magnitude() > 0.5 {
                            if rand::random_range(0..10000) == 0 {
                                blocks[index_in_chunk(x, y, z)] = GOLD.id;    
                            } else {
                                blocks[index_in_chunk(x, y, z)] = STONE.id;
                            }
                        } else if yf64+1.0 < height {
                            blocks[index_in_chunk(x, y, z)] = DIRT.id;
                        }else {
                            blocks[index_in_chunk(x, y, z)] = GRASS.id;
                        }
                    } else {
                        if yf64 <= 0.0 {
                            blocks[index_in_chunk(x, y, z)] = WATER.id;
                        } else {
                            blocks[index_in_chunk(x, y, z)] = AIR.id;
                        }
                    }
                }
            }
        }
        fn height_at(x: i32, z: i32, seed: u32) -> f64 {
            let noises = (0..8).map(|i| Perlin::new(seed+i)).collect::<Vec<_>>();
            let mut position_scale = 2000.0;
            let mut height_scale = 300.0;
            let mut height = 0.0;
            let xf64 = x as f64;
            let zf64 = z as f64;
            let mut noise_gradient = vec2(0.0, 0.0);
            for noise in noises {
                position_scale *= 0.6;
                height_scale *= 0.5;
                let noise_here = noise.get([xf64 / position_scale, zf64 / position_scale]);
                let noise_px = noise.get([(xf64+1.0) / position_scale, (zf64) / position_scale]);
                let noise_pz = noise.get([(xf64) / position_scale, (zf64+1.0) / position_scale]);
                noise_gradient += vec2(noise_here-noise_px, noise_here-noise_pz)/position_scale;
                height += noise_here * height_scale * 1.0/(1.0+noise_gradient.magnitude());
            }
            return height;
        }
        let mut features_placed = 0;
        fn place_feature(
            feature: impl Feature, attempts: usize, chance: f32, features_placed: &mut i32,
            seed: u32, req: &GenerateChunkBlocksRequest,
            noise_map: &Vec<f64>, noises: &Vec<Perlin>, blocks: &mut Vec<u16>,
        ) {
            let chunk_sizei32 = CHUNK_SIZE as i32;
            let chunk_size1 = CHUNK_SIZE as usize + 1;
            for cx in -1..2 {
                for cy in -1..2 {
                    for cz in -1..2 {
                        let mut hasher = DefaultHasher::new();
                        seed.hash(&mut hasher);
                        features_placed.hash(&mut hasher);
                        [req.chunk_position[0]+cx, req.chunk_position[1]+cy, req.chunk_position[2]+cz].hash(&mut hasher);
                        let feature_seed = hasher.finish();
                        let mut rand = SmallRng::seed_from_u64(feature_seed);
                        for _ in 0..attempts {
                            if rand.random_range(0.0..1.0) > chance {
                                continue;
                            }
                            let x = rand.random_range(0..chunk_sizei32)+cx*chunk_sizei32;
                            let z = rand.random_range(0..chunk_sizei32)+cz*chunk_sizei32;
                            let height = if x >= 0 && x < chunk_sizei32 && z >= 0 && z < chunk_sizei32 {
                                noise_map[x as usize * chunk_size1 * (noises.len()+1) + z as usize * (noises.len()+1) + noises.len()]
                            } else {
                                height_at(x+req.chunk_position[0]*chunk_sizei32, z+req.chunk_position[2]*chunk_sizei32, seed)
                            };
                            if height < 0.0 {
                                continue;
                            }
                            if height as i32 + 1 >= (req.chunk_position[1]+cy)*chunk_sizei32 && height as i32 + 1 < (req.chunk_position[1]+cy+1)*chunk_sizei32 {
                                let y = height as i32 + 1 - (req.chunk_position[1])*chunk_sizei32;
                                feature.place(x, y, z, &mut rand.fork(), |x: i32, y: i32, z: i32, block_id| {
                                    if x >= 0 && x < chunk_sizei32 && y >= 0 && y < chunk_sizei32 && z >= 0 && z < chunk_sizei32 {
                                        blocks[index_in_chunk(x as u32, y as u32, z as u32)] = block_id;
                                    }
                                });
                            }
                        }
                    }
                }
            }
            features_placed.add_assign(1);
        }
        place_feature(TreeFeature {}, 9, 0.7, &mut features_placed, seed, &req, &noise_map, &noises, &mut blocks);
        place_feature(BushFeature {}, 2, 0.9, &mut features_placed, seed, &req, &noise_map, &noises, &mut blocks);
        GenerateChunkBlocksResponse {
            chunk_position: req.chunk_position,
            chunk_blocks: blocks,
        }
    }

    fn generate_mesh_req(req: GenerateChunkMeshRequest, device: &Device) -> GenerateChunkModelResponse {
        if req.chunk_blocks == [0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize] {
            return GenerateChunkModelResponse {
                chunk_model: None,
                chunk_position: req.chunk_position,
            };
        }
        let lod = 2i32.pow((Vector3::<i32>::from(req.chunk_position).cast::<f32>().unwrap().mul(CHUNK_SIZE as f32).distance2(Vector3::<f32>::from(req.player_position))/500.0f32.powi(2).floor()) as u32).min(4).max(1);
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
                    return AIR.id;
                }
            }
        }
        let chunk_position = req.chunk_position;
        // let mut vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        // let mut transparency_vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        let mut vertices = Vec::with_capacity(8192);
        let mut transparency_vertices = Vec::with_capacity(8192);
        let mut modeled_vertices = Vec::with_capacity(256);
        let mut modeled_indices = Vec::with_capacity(256);
        let mut model_cache = HashMap::new();
        let mut mask = [AIR; (CHUNK_SIZE*CHUNK_SIZE) as usize];
        let mut block_cache: Vec<Option<Block>> = vec![];
        #[inline(always)]
        fn get_block_cached(block_id: BlockID, block_cache: &mut Vec<Option<Block>>) -> Block {
            if block_cache.len() > block_id as usize && let Some(block) = block_cache[block_id as usize] {
                return block;
            } else {
                let block = blocks::get_block(block_id);
                if block_cache.len() <= block_id as usize {
                    block_cache.extend_from_slice(&vec![None; block_id as usize + 1 - block_cache.len()]);
                }
                block_cache[block_id as usize] = Some(block);
                block
            }
        }
        #[inline(always)]
        fn add_modeled_block(block: Block, position: [i32; 3], chunk_position: [i32; 3], modeled_vertices: &mut Vec<Vertex>, modeled_indices: &mut Vec<u32>, model_cache: &mut HashMap<BlockID, BlockModel>) {
            let model = if let Some(model) = model_cache.get(&block.id) {
                model
            } else {
                let model = parse_model(block);
                model_cache.insert(block.id, model);
                model_cache.get(&block.id).unwrap()
            };
            let chunk_x_f32 = chunk_position[0] as f32 * CHUNK_SIZE as f32;
            let chunk_y_f32 = chunk_position[1] as f32 * CHUNK_SIZE as f32;
            let chunk_z_f32 = chunk_position[2] as f32 * CHUNK_SIZE as f32;
            let atlas_width_proportion = block.atlas_section.width as f32 / BLOCK_ATLAS_PNG_WIDTH as f32;
            let atlas_height_proportion = block.atlas_section.height as f32 / BLOCK_ATLAS_PNG_HEIGHT as f32; 
            modeled_vertices.extend(model.vertices.iter().map(|it| Vertex { position: [it.position[0]+position[0]as f32+chunk_x_f32, it.position[1]+position[1]as f32+chunk_y_f32, it.position[2]+position[2]as f32+chunk_z_f32, it.position[3]], color: [it.color[0]+block.atlas_section.x as f32 * atlas_width_proportion, it.color[1]+block.atlas_section.y as f32 * atlas_height_proportion, it.color[2], it.color[3]], normal: it.normal }));
            let num_indices = modeled_indices.len();
            modeled_indices.extend(model.indices.iter().map(|it| *it+num_indices as u32));
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
                            let block_here = get_block_cached(get_block(pos[0], pos[1], pos[2], &req, lod), &mut block_cache);
                            if block_here.has_model && lod == 1 {
                                add_modeled_block(block_here, pos, chunk_position, &mut modeled_vertices, &mut modeled_indices, &mut model_cache);
                                mask_i += 1;
                                continue;
                            }

                            let block_there = get_block_cached(get_block(pos[0]+slice_direction[0], pos[1]+slice_direction[1], pos[2]+slice_direction[2], &req, lod), &mut block_cache);

                            mask[mask_i] = if ((block_here.id != block_there.id) && (block_here.layer < block_there.layer || block_there.has_model)) || !block_here.cull || !block_there.cull {
                                block_here
                            } else {
                                AIR
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
                        vertices: &mut Vec<Vertex>, transparency_vertices: &mut Vec<Vertex>,
                        direction: i32, dim: usize, slice: i32,
                        chunk_position: [i32; 3],
                    ) {
                        let layer = block.layer;

                        let vertices_for = match layer {
                            0 => vertices,
                            1 => transparency_vertices,
                            _ => panic!()
                        };
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
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + (x+w) as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [-1.0, 0.0, 0.0, 0.0]
                                    },
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + (x+w) as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + (x+w) as f32, chunk_z_f32 + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + y as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + slice as f32+direction_f32, chunk_y_f32 + x as f32, chunk_z_f32 + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
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
                                        normal: [0.0, -1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, -1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (y+h) as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]],
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, -1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, -1.0, 0.0, 0.0]
                                    },
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (y+h) as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + y as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (y+h) as f32, chunk_y_f32 + slice as f32+direction_f32, chunk_z_f32 + x as f32, c[3]],
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    }
                                ]);
                            }
                        }
                        if dim == 2 {
                            if direction == -1 {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, -1.0, 0.0]
                                    },
                                    
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + (y+h) as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + (x+w) as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [chunk_x_f32 + x as f32, chunk_y_f32 + y as f32, chunk_z_f32 + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    }
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
                                while y+h < chunk_sizei32 && mask[mask_index(x, y+h)].id == block.id {
                                    h += lod;
                                }
                                'width: loop {
                                    if x+w >= chunk_sizei32 {
                                        break;
                                    }
                                    for i in (0..h).step_by(lod as usize) {
                                        if mask[mask_index(x+w, y+i)].id != block.id {
                                            break 'width;
                                        }
                                    }
                                    w += lod;
                                }
                                add_quad(x, y, w, h, block, &mut vertices, &mut transparency_vertices, direction, dim, slice, chunk_position);
                                for zero_x in x..x+w {
                                    for zero_y in y..y+h {
                                        mask[mask_index(zero_x, zero_y)] = AIR;
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

        let max_num_indices = vertices.len().max(transparency_vertices.len())/4;
        let mut indices = vec![0; max_num_indices*6];
        for i in 0..max_num_indices {
            indices[i*6+0] = i as u32 *4;
            indices[i*6+1] = i as u32 *4+3;
            indices[i*6+2] = i as u32 *4+2;
            indices[i*6+3] = i as u32 *4;
            indices[i*6+4] = i as u32 *4+1;
            indices[i*6+5] = i as u32 *4+3;
        }
        let vertex_size = (vertices.len() * size_of::<Vertex>()) as u64;
        let index_size = (max_num_indices*6 * size_of::<u32>()) as u64;

        let t_vertex_size = (transparency_vertices.len() * size_of::<Vertex>()) as u64;

        let m_vertex_size = (modeled_vertices.len() * size_of::<Vertex>()) as u64;
        let m_index_size = (modeled_indices.len() * size_of::<u32>()) as u64;
        let total_size = vertex_size + t_vertex_size + index_size + m_vertex_size + m_index_size;
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Chunk model"),
            mapped_at_creation: total_size > 0,
            size: total_size,
            usage: BufferUsages::VERTEX | BufferUsages::INDEX,
        });
        if total_size > 0 {
            let mut mapped = buffer
                .get_mapped_range_mut(..).unwrap();
            mapped
                .slice(..vertex_size as usize)
                .copy_from_slice(bytemuck::cast_slice(&vertices));
            mapped
                .slice((vertex_size) as usize..(vertex_size+t_vertex_size) as usize)
                .copy_from_slice(bytemuck::cast_slice(&transparency_vertices));
            mapped
                .slice((vertex_size+t_vertex_size) as usize..(vertex_size+t_vertex_size+index_size) as usize)
                .copy_from_slice(bytemuck::cast_slice(&indices));
            mapped
                .slice((vertex_size+t_vertex_size+index_size) as usize..(vertex_size+t_vertex_size+index_size+m_vertex_size) as usize)
                .copy_from_slice(bytemuck::cast_slice(&modeled_vertices));
            mapped
                .slice((vertex_size+t_vertex_size+index_size+m_vertex_size) as usize..(vertex_size+t_vertex_size+index_size+m_vertex_size+m_index_size) as usize)
                .copy_from_slice(bytemuck::cast_slice(&modeled_indices));
            drop(mapped);
            buffer.unmap();
        }
        let chunk_model = Some(ChunkModel {
            buffer,
            num_indices: vertices.len()/4*6,
            num_vertices: vertices.len(),
            num_transparent_indices: transparency_vertices.len()/4*6,
            num_transparent_vertices: transparency_vertices.len(),
            num_modeled_indices: modeled_indices.len(),
            num_modeled_vertices: modeled_vertices.len(),
        });
        
        
        GenerateChunkModelResponse {
            chunk_position,
            chunk_model,
        }
    }

    pub fn get_chunk_or_create(&mut self, chunk_position: [i32; 3]) -> &mut Chunk {
        let exists = self.chunks.get(chunk_position).is_some();
        if exists {
            return self.chunks.get_mut(chunk_position).unwrap();
        } else {
            self.chunks.set(chunk_position, Chunk::new());
            return self.chunks.get_mut(chunk_position).unwrap();
        }
    }

    // fn chunk_distance(chunk_position: &[i32; 3], camera_position: [f32; 3]) -> f32 {
    //     Self::chunk_distance2(chunk_position, camera_position).sqrt()
    // }

    fn chunk_distance2_by_world(chunk_position: &[i32; 3], camera_position: [f32; 3]) -> f32 {
        let chunk_position_world = [(chunk_position[0] as f32+0.5) * CHUNK_SIZE as f32 , (chunk_position[1] as f32+0.5) * CHUNK_SIZE as f32, (chunk_position[2] as f32+0.5) * CHUNK_SIZE as f32];
        (chunk_position_world[0]-camera_position[0]).powi(2)+(chunk_position_world[1]-camera_position[1]).powi(2)+(chunk_position_world[2]-camera_position[2]).powi(2)
    }

    fn chunk_distance2_by_chunk(chunk_position: &[i32; 3], camera_position: [f32; 3]) -> i32 {
        let camera_position_chunk = [camera_position[0] as i32 / CHUNK_SIZE as i32, camera_position[1] as i32 / CHUNK_SIZE as i32, camera_position[2] as i32 / CHUNK_SIZE as i32];
        (camera_position_chunk[0]-chunk_position[0]).pow(2)+(camera_position_chunk[1]-chunk_position[1]).pow(2)+(camera_position_chunk[2]-chunk_position[2]).pow(2)
    }

    pub fn chunks_sorted(&self, camera_position: [f32; 3]) -> impl Iterator<Item = &Chunk> {
        self.chunks.main.keys().sorted_by(|chunk_position_a, chunk_position_b| {
            let da = Self::chunk_distance2_by_world(chunk_position_a, camera_position);
            let db = Self::chunk_distance2_by_world(chunk_position_b, camera_position);
            db.partial_cmp(&da).unwrap()
        }).map(|i| self.chunks.main.get(i).unwrap())
    }

    pub fn chunks(&self) -> impl Iterator<Item = (&[i32; 3], &Chunk)> {
        self.chunks.main.iter()
    }

    pub fn chunks_mut(&mut self) -> impl Iterator<Item = (&[i32; 3], &mut Chunk)> {
        self.chunks.main.iter_mut()
    }

    pub fn chunk_positions_owned(&self) -> Vec<[i32; 3]> {
        self.chunks.main.keys().cloned().collect()
    }

    pub fn chunk_positions(&self) -> std::collections::hash_map::Keys<'_, [i32; 3], Chunk> {
        self.chunks.main.keys()
    }

    pub fn chunk_exists(&self, chunk_position: [i32; 3]) -> bool {
        self.chunks.main.contains_key(&chunk_position)
    }

    pub fn chunk_loaded(&self, chunk_position: [i32; 3]) -> bool {
        if self.chunks.main.contains_key(&chunk_position) {
            return self.chunks.main.get(&chunk_position).unwrap().generated_blocks;
        } else {
            return false;
        }
    }

    pub fn get_block(&self, world_position: [i32; 3]) -> BlockID {
        let x = world_position[0];
        let y = world_position[1];
        let z = world_position[2];
        let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
        let bx = x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let by = y.rem_euclid(CHUNK_SIZE as i32) as u32;
        let bz = z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if let Some(chunk) = self.chunks.get([cx, cy, cz]) {
            return chunk.blocks[index_in_chunk(bx, by, bz)];
        } else {
            return AIR.id;
        }
    }

    pub fn set_block(&mut self, world_position: [i32; 3], block_id: BlockID, update_synchronously: bool) {
        let x = world_position[0];
        let y = world_position[1];
        let z = world_position[2];
        let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
        let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
        let bx = x.rem_euclid(CHUNK_SIZE as i32) as u32;
        let by = y.rem_euclid(CHUNK_SIZE as i32) as u32;
        let bz = z.rem_euclid(CHUNK_SIZE as i32) as u32;
        if let Some(chunk) = self.chunks.get_mut([cx, cy, cz]) {
            chunk.blocks[index_in_chunk(bx, by, bz)] = block_id;
            chunk.set_block([bx, by, bz], block_id, update_synchronously);
        }
    }
}

struct ChunkMap {
    main: HashMap<[i32; 3], Chunk>,
}

impl ChunkMap {
    fn new() -> Self {
        Self {
            main: HashMap::new(),
        }
    }

    fn get(&self, key: [i32; 3]) -> Option<&Chunk> {
        self.main.get(&key)
    }

    fn get_mut(&mut self, key: [i32; 3]) -> Option<&mut Chunk> {
        self.main.get_mut(&key)
    }

    fn set(&mut self, key: [i32; 3], chunk: Chunk) -> Option<Chunk> {
        self.main.insert(key, chunk)
    }
}