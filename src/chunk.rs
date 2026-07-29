use std::{collections::HashMap, ops::AddAssign, sync::mpsc, time::{Duration, SystemTime}};

use bespoke_engine::surface_context::SurfaceCtx;
use cgmath::{InnerSpace, MetricSpace, Vector3, vec2};
use noise::{NoiseFn, Perlin};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, Device, RenderPass, util::{BufferInitDescriptor, DeviceExt}};
use itertools::Itertools;

use crate::{blocks::{self, AIR, ATLAS_X_BLOCKS, ATLAS_Y_BLOCKS, Block, BlockID, DIRT, GRASS, NOT_RENDERED_LAYER, STONE, WATER, block_layer}, game::Vertex, util::neighbors};

pub struct Chunk {
    blocks: Vec<BlockID>,
    model: Option<ChunkModel>,
    transparency_model: Option<ChunkModel>,
}

pub struct ChunkModel {
    // vertices: Buffer,
    num_vertices: usize,
    buffer: Buffer,
    num_indices: usize,
}

pub const CHUNK_SIZE: u32 = 32;

pub fn index_in_chunk(x: u32, y: u32, z: u32) -> usize {
    (y * CHUNK_SIZE * CHUNK_SIZE + x * CHUNK_SIZE + z) as usize
}

impl Chunk {
    pub fn new() -> Self {
        let mut _self = Self {
            blocks: vec![0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize],
            model: None,
            transparency_model: None,
        };
        _self
    }

    pub fn set_block(&mut self, local_coords: [u32; 3], block: BlockID) {
        self.blocks[index_in_chunk(local_coords[0], local_coords[1], local_coords[2])] = block;
    }

    pub fn render(&self, render_pass: &mut RenderPass) {
        // if let Some(model) = &self.model {
        //     if model.num_vertices > 0 {
        //         render_pass.set_vertex_buffer(0, model.vertices.slice(..));
        //         render_pass.set_index_buffer(model.indices.slice(..), wgpu::IndexFormat::Uint32);
        //         render_pass.draw_indexed(0..model.num_indices as u32, 0, 0..1);
        //     }
        // }
        if let Some(model) = &self.transparency_model {
            if model.num_vertices > 0 {
                render_pass.set_vertex_buffer(0, model.buffer.slice(..(model.num_vertices*size_of::<Vertex>()) as u64));
                render_pass.set_index_buffer(model.buffer.slice((model.num_vertices*size_of::<Vertex>()) as u64..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model.num_indices as u32, 0, 0..1);
            }
        }
    }

    pub fn visible(&self) -> bool {
        self.model.as_ref().is_some_and(|it| it.num_vertices > 0) || self.transparency_model.as_ref().is_some_and(|it| it.num_vertices > 0)
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
    transparency_model: Option<ChunkModel>,
}

pub struct ChunkManager {
    chunks: ChunkMap,
    gen_blocks_tx: mpsc::Sender<GenerateChunkBlocksRequest>,
    gen_blocks_rx: mpsc::Receiver<GenerateChunkBlocksResponse>,
    gen_model_tx: mpsc::Sender<GenerateChunkMeshRequest>,
    gen_model_rx: mpsc::Receiver<GenerateChunkModelResponse>,
}

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
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2(&it.chunk_position, player_position) < Self::chunk_distance2(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
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
                    if n % 100 == 99 {
                        let avg = t/n;
                        println!("avg gen blocks: {avg:?}");
                    }
                    while let Ok(req) = gen_blocks_req_rx.try_recv() {
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2(&it.chunk_position, player_position) < Self::chunk_distance2(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
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
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2(&it.chunk_position, player_position) > Self::chunk_distance2(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
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
                    if n % 100 == 99 {
                        let avg = t/n;
                        println!("avg gen mesh: {avg:?}");
                        t = Duration::ZERO;
                        n = 0;
                    }
                    while let Ok(req) = gen_model_req_rx.try_recv() {
                        queue.insert(queue.iter().find_position(|it| Self::chunk_distance2(&it.chunk_position, player_position) > Self::chunk_distance2(&req.chunk_position, player_position)).map(|it| it.0).unwrap_or(0), req);
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
                self.generate_model(res.chunk_position, player_position);
                for chunk_position in neighbors(res.chunk_position) {
                    self.generate_model(chunk_position, player_position);
                }
            }
        }
        while let Ok(res) = self.gen_model_rx.try_recv() {
            if let Some(chunk) = self.chunks.get_mut(res.chunk_position) {
                chunk.model = res.chunk_model;
                chunk.transparency_model = res.transparency_model;
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
        let noise = (0..6).map(|i| Perlin::new(seed+i)).collect::<Vec<_>>();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let mut position_scale = 2000.0;
                let mut height_scale = 300.0;
                let mut height = 0.0;
                let xf64 = x as f64 + req.chunk_position[0] as f64 * CHUNK_SIZE as f64;
                let zf64 = z as f64 + req.chunk_position[2] as f64 * CHUNK_SIZE as f64;
                let mut gradient = vec2(0.0, 0.0);
                for noise in &noise {
                    position_scale *= 0.6;
                    height_scale *= 0.5;
                    let noise_here = noise.get([xf64 / position_scale, zf64 / position_scale]);
                    let noise_px = noise.get([(xf64+1.0) / position_scale, (zf64) / position_scale]);
                    let noise_pz = noise.get([(xf64) / position_scale, (zf64+1.0) / position_scale]);
                    gradient += vec2(noise_here-noise_px, noise_here-noise_pz);
                    height += noise_here * height_scale * 1.0/(1.0+gradient.magnitude());
                }
                for y in 0..CHUNK_SIZE {
                    let yf64 = y as f64+req.chunk_position[1] as f64 *(CHUNK_SIZE as f64);
                    if yf64 <= height {
                        if yf64+5.0 < height || gradient.magnitude() > 0.04 {
                            blocks[index_in_chunk(x, y, z)] = STONE.id;
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
        GenerateChunkBlocksResponse {
            chunk_position: req.chunk_position,
            chunk_blocks: blocks,
        }
    }

    fn generate_mesh_req_not_greedy(req: GenerateChunkMeshRequest, device: &Device) -> GenerateChunkModelResponse {
        if req.chunk_blocks == [0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize] {
            return GenerateChunkModelResponse {
                chunk_model: None,
                transparency_model: None,
                chunk_position: req.chunk_position,
            };
        }
        let chunk_position = req.chunk_position;
        let mut vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        let mut transparency_vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        let mut num_vertices = 0;
        let mut num_transparency_vertices = 0;
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let block = req.chunk_blocks[index_in_chunk(x, y, z)];
                    let layer_here = block_layer(block);
                    if layer_here < NOT_RENDERED_LAYER {
                        let vertices_for = match layer_here {
                            0 => &mut vertices,
                            1 => &mut transparency_vertices,
                            _ => panic!()
                        };
                        let num_vertices_for = match layer_here {
                            0 => &mut num_vertices,
                            1 => &mut num_transparency_vertices,
                            _ => panic!()
                        };
                        let get_block = |x: i32, y: i32, z: i32| -> BlockID {
                            if x >= 0 && y >= 0 && z >= 0 && x < CHUNK_SIZE as i32 && y < CHUNK_SIZE as i32 && z < CHUNK_SIZE as i32 {
                                return req.chunk_blocks[index_in_chunk(x as u32, y as u32, z as u32)];
                            } else {
                                let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
                                let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
                                let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
                                let bx = x.rem_euclid(CHUNK_SIZE as i32);
                                let by = y.rem_euclid(CHUNK_SIZE as i32);
                                let bz = z.rem_euclid(CHUNK_SIZE as i32);
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
                                    return blocks[index_in_chunk(bx as u32, by as u32, bz as u32)];
                                } else {
                                    return AIR.id;
                                }
                            }
                        };
                        let block_inst = blocks::get_block(block);
                        let c = block_inst.color;
                        let atlas_x = block_inst.atlas_x;
                        let atlas_y = block_inst.atlas_y;
                        let tex_width_x = 1.0/ATLAS_X_BLOCKS as f32;
                        let tex_width_y = 1.0/ATLAS_Y_BLOCKS as f32;
                        let tex_coords_offset_x = tex_width_x * atlas_x as f32;
                        let tex_coords_offset_y = tex_width_y * atlas_y as f32;
                        let cpx = get_block(x as i32+1, y as i32, z as i32);
                        let cnx = get_block(x as i32-1, y as i32, z as i32);
                        let cpy = get_block(x as i32, y as i32+1, z as i32);
                        let cny = get_block(x as i32, y as i32-1, z as i32);
                        let cpz = get_block(x as i32, y as i32, z as i32+1);
                        let cnz = get_block(x as i32, y as i32, z as i32-1);
                        if block_layer(cpx) > layer_here {
                            vertices_for[*num_vertices_for+0] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [1.0, 0.0, 0.0, 0.0],
                            };
                            vertices_for[*num_vertices_for+1] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [1.0, 0.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+2] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [1.0, 0.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+3] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [1.0, 0.0, 0.0, 0.0]
                            };
                            num_vertices_for.add_assign(4);
                        }
                        if block_layer(cnx) > layer_here {
                            vertices_for[*num_vertices_for+0] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [-1.0, 0.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+1] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [-1.0, 0.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+2] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [-1.0, 0.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+3] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [-1.0, 0.0, 0.0, 0.0]
                            };
                            num_vertices_for.add_assign(4);
                        }
                        if block_layer(cpy) > layer_here {
                            vertices_for[*num_vertices_for+0] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, 1.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+1] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, 1.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+2] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, 1.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+3] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0],
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, 1.0, 0.0, 0.0]
                            };
                            num_vertices_for.add_assign(4);
                        }
                        if block_layer(cny) > layer_here {
                            vertices_for[*num_vertices_for+0] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, -1.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+1] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, -1.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+2] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, -1.0, 0.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+3] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, -1.0, 0.0, 0.0]
                            };
                            num_vertices_for.add_assign(4);
                        }
                        if block_layer(cpz) > layer_here {
                            vertices_for[*num_vertices_for+0] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, 0.0, 1.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+1] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, 0.0, 1.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+2] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, 0.0, 1.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+3] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32+1.0, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, 0.0, 1.0, 0.0]
                            };
                            num_vertices_for.add_assign(4);
                        }
                        if block_layer(cnz) > layer_here {
                            vertices_for[*num_vertices_for+0] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, 0.0, -1.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+1] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32+1.0, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, c[2], c[3]], 
                                normal: [0.0, 0.0, -1.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+2] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, 0.0, -1.0, 0.0]
                            };
                            vertices_for[*num_vertices_for+3] = Vertex {
                                position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32+1.0, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + z as f32, 0.0], 
                                color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, c[2], c[3]], 
                                normal: [0.0, 0.0, -1.0, 0.0]
                            };
                            num_vertices_for.add_assign(4);
                        }
                    }
                }
            }
        }
        let chunk_model = {
            let mut indices = vec![0; num_vertices/4*6];
            for i in 0..num_vertices/4 {
                indices[i*6+0] = i as u32 *4;
                indices[i*6+1] = i as u32 *4+3;
                indices[i*6+2] = i as u32 *4+2;
                indices[i*6+3] = i as u32 *4;
                indices[i*6+4] = i as u32 *4+1;
                indices[i*6+5] = i as u32 *4+3;
            }
            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&vertices[0..num_vertices]),
                label: Some("Chunk vertex buffer"),
                usage: BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&indices),
                label: Some("Chunk index buffer"),
                usage: BufferUsages::INDEX,
            });
            Some(ChunkModel {
                vertices: vertex_buffer,
                indices: index_buffer,
                num_indices: indices.len(),
                num_vertices: num_vertices,
            })
        };
        let transparency_model = {
            let mut indices = vec![0; num_transparency_vertices/4*6];
            for i in 0..num_transparency_vertices/4 {
                indices[i*6+0] = i as u32 *4;
                indices[i*6+1] = i as u32 *4+3;
                indices[i*6+2] = i as u32 *4+2;
                indices[i*6+3] = i as u32 *4;
                indices[i*6+4] = i as u32 *4+1;
                indices[i*6+5] = i as u32 *4+3;
            }
            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&transparency_vertices[0..num_transparency_vertices]),
                label: Some("Chunk vertex buffer"),
                usage: BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&indices),
                label: Some("Chunk index buffer"),
                usage: BufferUsages::INDEX,
            });
            Some(ChunkModel {
                vertices: vertex_buffer,
                indices: index_buffer,
                num_indices: indices.len(),
                num_vertices: num_transparency_vertices,
            })
        };
        
        
        GenerateChunkModelResponse {
            chunk_position,
            chunk_model,
            transparency_model,
        }
    }

    fn generate_mesh_req(req: GenerateChunkMeshRequest, device: &Device) -> GenerateChunkModelResponse {
        if req.chunk_blocks == [0; (CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE) as usize] {
            return GenerateChunkModelResponse {
                chunk_model: None,
                transparency_model: None,
                chunk_position: req.chunk_position,
            };
        }
        let lod = 2i32.pow((Vector3::<i32>::from(req.chunk_position).cast::<f32>().unwrap().distance2(Vector3::<f32>::from(req.player_position))/10.0f32.powi(2).floor()) as u32).min(4).max(1);
        let get_block = |x: i32, y: i32, z: i32| -> BlockID {
            if x >= 0 && y >= 0 && z >= 0 && x < CHUNK_SIZE as i32 && y < CHUNK_SIZE as i32 && z < CHUNK_SIZE as i32 {
                if lod == 1 {
                    return req.chunk_blocks[index_in_chunk(x as u32, y as u32, z as u32)];
                } else {
                    let lx = x & !(lod - 1);
                    let ly = y & !(lod - 1);
                    let lz = z & !(lod - 1);
                    return req.chunk_blocks[index_in_chunk(lx as u32, ly as u32, lz as u32)];
                }
            } else {
                let cx = (x as f32 /CHUNK_SIZE as f32).floor() as i32;
                let cy = (y as f32 /CHUNK_SIZE as f32).floor() as i32;
                let cz = (z as f32 /CHUNK_SIZE as f32).floor() as i32;
                let bx = x.rem_euclid(CHUNK_SIZE as i32);
                let by = y.rem_euclid(CHUNK_SIZE as i32);
                let bz = z.rem_euclid(CHUNK_SIZE as i32);
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
        };
        let chunk_position = req.chunk_position;
        // let mut vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        // let mut transparency_vertices = vec![Vertex { normal: [0.0; 4], position: [0.0; 4], color: [0.0; 4] }; CHUNK_SIZE as usize*CHUNK_SIZE as usize*CHUNK_SIZE as usize*24];
        let mut vertices = Vec::with_capacity(8192);
        let mut transparency_vertices = Vec::with_capacity(8192);
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
        for direction in [-1, 1] {
            for dim in 0..3 {
                let mut slice_direction = [0; 3];
                slice_direction[dim] = direction;
                for slice in 0..CHUNK_SIZE as i32 {
                    let mut mask_i = 0;
                    let u = (dim+1)%3;
                    let v = (dim+2)%3;
                    for x in 0..CHUNK_SIZE as i32 {
                        for y in 0..CHUNK_SIZE as i32 {
                            let mut pos = [0; 3];
                            pos[dim] = slice;
                            pos[u] = x;
                            pos[v] = y;
                            let block_here = get_block_cached(get_block(pos[0], pos[1], pos[2]), &mut block_cache);
                            let block_there = get_block_cached(get_block(pos[0]+slice_direction[0], pos[1]+slice_direction[1], pos[2]+slice_direction[2]), &mut block_cache);
                            
                            mask[mask_i] = if block_here.id != block_there.id && block_here.layer() < block_there.layer() {
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
                    let mut add_quad = |x: i32, y: i32, w: i32, h: i32, block: Block| {
                        let layer = block.layer();

                        let vertices_for = match layer {
                            0 => &mut vertices,
                            1 => &mut transparency_vertices,
                            _ => panic!()
                        };
                        // let num_vertices_for = match layer {
                        //     0 => &mut num_vertices,
                        //     1 => &mut num_transparency_vertices,
                        //     _ => panic!()
                        // };

                        let c = block.color;
                        let atlas_x = block.atlas_x;
                        let atlas_y = block.atlas_y;
                        let tex_width_x = 1.0/ATLAS_X_BLOCKS as f32;
                        let tex_width_y = 1.0/ATLAS_Y_BLOCKS as f32;
                        let tex_coords_offset_x = tex_width_x * atlas_x as f32;
                        let tex_coords_offset_y = tex_width_y * atlas_y as f32;

                        let direction_f32 = (direction as f32 + 1.0)/2.0;
                        if dim == 0 {
                            if direction == -1 {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + y as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + y as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
                                    },
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + y as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0],
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + y as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [1.0, 0.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, c[3]], 
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
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + x as f32, c[3]],
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + x as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + x as f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, h as f32, w as f32], 
                                        normal: [0.0, 1.0, 0.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + x as f32, c[3]],
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
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    
                                ]);
                            } else {
                                vertices_for.extend_from_slice(&[
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + (y+h) as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y+tex_width_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + (x+w) as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x+tex_width_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    },
                                    Vertex {
                                        position: [(chunk_position[0] as f32 * CHUNK_SIZE as f32) + x as f32, (chunk_position[1] as f32 * CHUNK_SIZE as f32) + y as f32, (chunk_position[2] as f32 * CHUNK_SIZE as f32) + slice as f32+direction_f32, c[3]], 
                                        color: [tex_coords_offset_x, tex_coords_offset_y, w as f32, h as f32], 
                                        normal: [0.0, 0.0, 1.0, 0.0]
                                    }
                                ]);
                            }
                        }
                    };

                    let mut x = 0;
                    let mut y = 0;
                    while x < CHUNK_SIZE as i32 {
                        while y < CHUNK_SIZE as i32 {
                            let block = mask[mask_index(x, y)];
                            let mut h = 0;
                            let mut w = lod;
                            if block.layer() != NOT_RENDERED_LAYER {
                                while y+h < CHUNK_SIZE as i32 && mask[mask_index(x, y+h)].id == block.id {
                                    h += lod;
                                }
                                'width: loop {
                                    if x+w >= CHUNK_SIZE as i32 {
                                        break;
                                    }
                                    for i in (0..h).step_by(lod as usize) {
                                        if mask[mask_index(x+w, y+i)].id != block.id {
                                            break 'width;
                                        }
                                    }
                                    w += lod;
                                }
                                add_quad(x, y, w, h, block);
                                for zero_x in x..x+w {
                                    for zero_y in y..y+h {
                                        mask[mask_index(zero_x, zero_y)] = AIR;
                                    }
                                }
                            }
                            y += lod;
                        }
                        if y >= CHUNK_SIZE as i32 {
                            y = 0;
                            x += lod;
                        }
                    }
                }
            }
        }
        let chunk_model = {
            let mut indices = vec![0; vertices.len()/4*6];
            for i in 0..vertices.len()/4 {
                indices[i*6+0] = i as u32 *4;
                indices[i*6+1] = i as u32 *4+3;
                indices[i*6+2] = i as u32 *4+2;
                indices[i*6+3] = i as u32 *4;
                indices[i*6+4] = i as u32 *4+1;
                indices[i*6+5] = i as u32 *4+3;
            }
            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&vertices),
                label: Some("Chunk vertex buffer"),
                usage: BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&indices),
                label: Some("Chunk index buffer"),
                usage: BufferUsages::INDEX,
            });
            Some(ChunkModel {
                vertices: vertex_buffer,
                indices: index_buffer,
                num_indices: indices.len(),
                num_vertices: vertices.len(),
            })
        };
        let transparency_model = {
            let mut indices = vec![0; transparency_vertices.len()/4*6];
            for i in 0..transparency_vertices.len()/4 {
                indices[i*6+0] = i as u32 *4;
                indices[i*6+1] = i as u32 *4+3;
                indices[i*6+2] = i as u32 *4+2;
                indices[i*6+3] = i as u32 *4;
                indices[i*6+4] = i as u32 *4+1;
                indices[i*6+5] = i as u32 *4+3;
            }
            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&transparency_vertices),
                label: Some("Chunk vertex buffer"),
                usage: BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                contents: bytemuck::cast_slice(&indices),
                label: Some("Chunk index buffer"),
                usage: BufferUsages::INDEX,
            });
            let vertex_size = (transparency_vertices.len() * size_of::<Vertex>()) as u64;
            let index_size = (indices.len() * size_of::<u32>()) as u64;
            let buffer = device.create_buffer(&BufferDescriptor {
                label: Some("Chunk transparency model"),
                mapped_at_creation: true,
                size: vertex_size + index_size,
                usage: BufferUsages::VERTEX | BufferUsages::INDEX,
            });
            buffer
                .get_mapped_range_mut(..)
                .slice(..vertex_size as usize)
                .copy_from_slice(bytemuck::cast_slice(&transparency_vertices));
            buffer
                .get_mapped_range_mut(..)
                .slice(vertex_size as usize..index_size as usize)
                .copy_from_slice(bytemuck::cast_slice(&indices));
            buffer.unmap();
            Some(ChunkModel {
                vertices: vertex_buffer,
                indices: index_buffer,
                num_indices: indices.len(),
                num_vertices: transparency_vertices.len(),
            })
        };
        
        
        GenerateChunkModelResponse {
            chunk_position,
            chunk_model,
            transparency_model,
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

    fn chunk_distance(chunk_position: &[i32; 3], camera_position: [f32; 3]) -> f32 {
        let chunk_position_world = [chunk_position[0] as f32 * CHUNK_SIZE as f32, chunk_position[1] as f32 * CHUNK_SIZE as f32, chunk_position[2] as f32 * CHUNK_SIZE as f32];
        ((chunk_position_world[0]-camera_position[0]).powi(2)+(chunk_position_world[1]-camera_position[1]).powi(2)+(chunk_position_world[2]-camera_position[2]).powi(2)).sqrt()
    }

    fn chunk_distance2(chunk_position: &[i32; 3], camera_position: [f32; 3]) -> f32 {
        let chunk_position_world = [chunk_position[0] as f32 * CHUNK_SIZE as f32, chunk_position[1] as f32 * CHUNK_SIZE as f32, chunk_position[2] as f32 * CHUNK_SIZE as f32];
        (chunk_position_world[0]-camera_position[0]).powi(2)+(chunk_position_world[1]-camera_position[1]).powi(2)+(chunk_position_world[2]-camera_position[2]).powi(2)
    }

    pub fn chunks(&mut self, camera_position: [f32; 3]) -> impl Iterator<Item = &Chunk> {
        self.chunks.main.keys().sorted_by(|chunk_position_a, chunk_position_b| {
            let da = Self::chunk_distance(chunk_position_a, camera_position);
            let db = Self::chunk_distance(chunk_position_b, camera_position);
            db.partial_cmp(&da).unwrap()
        }).map(|i| self.chunks.main.get(i).unwrap())
    }

    pub fn chunk_positions(&self) -> std::collections::hash_map::Keys<'_, [i32; 3], Chunk> {
        self.chunks.main.keys()
    }

    pub fn chunk_exists(&self, chunk_position: [i32; 3]) -> bool {
        self.chunks.main.contains_key(&chunk_position)
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