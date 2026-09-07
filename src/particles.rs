use std::time::Duration;

use bespoke_engine::{InstanceTrait, binding::{Descriptor, UniformBinding}, compute::{ComputeOutput, ComputeShader}, culling::AABB, model::{Model, Render, ToRaw}, resource_loader::load_resource_string, shader::{Shader, ShaderType, UniformShaderInit}, surface_context::SurfaceCtx, window::BasicVertex};
use bytemuck::{Pod, Zeroable, bytes_of, checked::from_bytes};
use cgmath::Vector3;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, RenderPass, ShaderStages, TextureFormat, wgt::BufferDescriptor};

use crate::{RES_SHADERS_PARTICLE_RENDERER_WGSL, game::ScreenInfo};

pub struct ParticleManager<'a> {
    particles_buffer: Buffer,
    particles_bind_group: BindGroup,
    particles_buffer2: Buffer,
    particles2_bind_group: BindGroup,
    delta_seconds_uniform: UniformBinding<f32>,
    flip: bool,
    pending_additions: Vec<Particle>,
    instances_buffer: Buffer,
    instances_bind_group: BindGroup,
    num_instances: u32,
    num_instances_output: ComputeOutput,
    particle_model: Model,

    update_shader: ComputeShader,
    instance_shader: ComputeShader,
    render_shader: Shader<'a>
}

impl <'a> ParticleManager<'a> {
    pub fn new(surface_ctx: &dyn SurfaceCtx, screen_info_binding: &UniformBinding<ScreenInfo>, deferred_formats: Vec<TextureFormat>) -> Self {
        let max_particles = 8000;
        let particles_buffer = surface_ctx.device().create_buffer(&BufferDescriptor {
            label: Some("Particles buffer"),
            mapped_at_creation: false,
            size: max_particles * size_of::<ParticleRaw>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });
        let particles_layout = surface_ctx.device().create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Particles Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                    visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
                },
            ]
        });
        let particles_bind_group = surface_ctx.device().create_bind_group(&BindGroupDescriptor {
            label: Some("Particles bind group"),
            layout: &particles_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: particles_buffer.as_entire_binding(),
                }
            ]
        });
        let particles_buffer2 = surface_ctx.device().create_buffer(&BufferDescriptor {
            label: Some("Particles 2 buffer"),
            mapped_at_creation: false,
            size: max_particles * size_of::<ParticleRaw>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });
        let particles2_bind_group = surface_ctx.device().create_bind_group(&BindGroupDescriptor {
            label: Some("Particles 2 bind group"),
            layout: &particles_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: particles_buffer2.as_entire_binding(),
                }
            ]
        });
        let instances_buffer = surface_ctx.device().create_buffer(&BufferDescriptor {
            label: Some("Particles instances buffer"),
            mapped_at_creation: false,
            size: max_particles * size_of::<ParticleInstanceRaw>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let instances_bind_group = surface_ctx.device().create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &particles_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instances_buffer.as_entire_binding(),
                },
            ]
        });

        let delta_seconds_uniform = UniformBinding::new(surface_ctx.device(), "Delta Seconds", 0.0, None);

        let num_instances_output = ComputeOutput::new(size_of::<u32>() as u64, &surface_ctx.device());

        let update_shader = ComputeShader::new(&load_resource_string("res/shaders/particle_update_shader.wgsl"), vec![&particles_layout, &particles_layout, &delta_seconds_uniform.layout], vec![&ShaderType::buffer_type(true, "Particle".into()), &ShaderType::buffer_type(true, "Particle".into()), &delta_seconds_uniform.shader_type], vec![], surface_ctx.device());
        let instance_shader = ComputeShader::new(&load_resource_string("res/shaders/particle_instance_shader.wgsl"), vec![&particles_layout, &particles_layout, &num_instances_output.layout], vec![&ShaderType::buffer_type(true, "Particle".into()), &ShaderType::buffer_type(true, "ParticleInstance".into())], vec![], surface_ctx.device());
        let render_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_PARTICLE_RENDERER_WGSL, formats: deferred_formats, uniforms: vec![screen_info_binding], vertex_buffers: vec![BasicVertex::desc(), ParticleInstance::desc()], ..Default::default() }, surface_ctx.device());

        let size = 0.1;
        let particle_model = Model::new(vec![
            BasicVertex { position: [-size, -size, 0.0], tex_coords: [0.0, 1.0] },
            BasicVertex { position: [-size, size, 0.0], tex_coords: [0.0, 0.0] },
            BasicVertex { position: [size, -size, 0.0], tex_coords: [1.0, 1.0] },
            BasicVertex { position: [size, size, 0.0], tex_coords: [1.0, 0.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB { dimensions: [1.0, 1.0, 0.0] }, surface_ctx.device());
        Self {
            instances_buffer,
            instance_shader,
            instances_bind_group,
            delta_seconds_uniform,
            num_instances: 0,
            num_instances_output,
            particles_buffer,
            particles_bind_group,
            particles_buffer2,
            particles2_bind_group,
            pending_additions: vec![],
            render_shader,
            update_shader,
            particle_model,
            flip: false,
        }
    }

    pub fn run_step(&mut self, delta_time: Duration, surface_ctx: &dyn SurfaceCtx) {
        self.add_pending_particles(surface_ctx);
        let in_buffer = if self.flip {
            &self.particles2_bind_group
        } else {
            &self.particles_bind_group
        };
        let out_buffer = if self.flip {
            &self.particles_bind_group
        } else {
            &self.particles2_bind_group
        };
        self.delta_seconds_uniform.set_data(surface_ctx.queue(), delta_time.as_secs_f32());
        self.update_shader.run_once(vec![in_buffer, out_buffer, &self.delta_seconds_uniform.binding], [20; 3], surface_ctx.device(), surface_ctx.queue());
        self.instance_shader.run_once(vec![&out_buffer, &self.instances_bind_group, &self.num_instances_output.binding], [20; 3], surface_ctx.device(), surface_ctx.queue());
        self.num_instances = *bytemuck::from_bytes::<u32>(&self.num_instances_output.read(surface_ctx.device(), surface_ctx.queue()));
        surface_ctx.queue().write_buffer(&self.num_instances_output.buffer, 0, &vec![0; self.num_instances_output.buffer.size() as usize]);
        self.flip = !self.flip;
    }

    pub fn render<'s: 'b, 'b>(&'s mut self, screen_info_binding: &UniformBinding<ScreenInfo>, render_pass: &mut RenderPass<'b>) {
        self.render_shader.bind(render_pass);
        render_pass.set_bind_group(0, &screen_info_binding.binding, &[]);
        self.particle_model.render_instances(render_pass, &self.instances_buffer, 0..self.num_instances);
    }

    pub fn add_particle(&mut self, particle: Particle) {
        self.pending_additions.push(particle);
    }

    pub fn add_pending_particles(&mut self, surface_ctx: &dyn SurfaceCtx) {
        let in_buffer = if self.flip {
            &self.particles_buffer2
        } else {
            &self.particles_buffer
        };
        let mut encoder = surface_ctx.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let map_buffer = surface_ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Output Map Buffer"),
            size: in_buffer.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(in_buffer, 0, &map_buffer, 0, in_buffer.size());
        surface_ctx.queue().submit([encoder.finish()]);
        map_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |result| {
                result.unwrap();
            });
        surface_ctx.device().poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let bytes = map_buffer.slice(..).get_mapped_range().unwrap().to_vec();
        map_buffer.unmap();
        let mut cursor = 0;
        'p: while let Some(particle) = self.pending_additions.pop() {
            while cursor < in_buffer.size() as usize {
                if *from_bytes::<u32>(&bytes[cursor+size_of::<[f32; 1]>()..cursor+size_of::<[f32; 1]>()+4]) == 0 {
                    let raw = ParticleRaw {
                        particle_type: particle.particle_type.shader_u32(),
                        color: particle.color,
                        lifetime: particle.lifetime.as_secs_f32(),
                        position: particle.position,
                        velocity: [particle.velocity[0], particle.velocity[1], particle.velocity[2], 0.0],
                        padding: [0.0; 2],
                    };
                    surface_ctx.queue().write_buffer(in_buffer, cursor as u64, bytes_of(&raw));  
                    cursor += size_of::<ParticleRaw>();
                    continue 'p;
                }
                cursor += size_of::<ParticleRaw>();
            }
        }
    }
}

pub struct Particle {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub lifetime: Duration,
    pub velocity: [f32; 3],
    pub particle_type: ParticleType,
}

#[derive(Pod, Clone, Copy, Zeroable, Debug)]
#[repr(C)]
struct ParticleRaw {
    lifetime: f32,
    particle_type: u32,
    padding: [f32; 2],
    position: [f32; 4],
    color: [f32; 4],
    velocity: [f32; 4],
}

pub enum ParticleType {
    BlockBreak,
}

impl ParticleType {
    pub fn shader_u32(&self) -> u32 {
        match self {
            ParticleType::BlockBreak => 1,
        }
    }
}

#[derive(Clone)]
pub struct ParticleInstance {
    pub position: cgmath::Vector3<f32>,
    pub color: [f32; 4],
}

impl InstanceTrait for ParticleInstance {
    fn instance_transform(&self) -> cgmath::Matrix4<f32> {
        cgmath::Matrix4::from_translation(self.position)
    }
}

impl ParticleInstance {
    pub fn raw(&self) -> ParticleInstanceRaw {
        ParticleInstanceRaw { position: self.position.extend(1.0).into(), color: self.color }
    }
}

impl Default for ParticleInstance {
    fn default() -> Self {
        Self { position: Vector3::new(0.0, 0.0, 0.0), color: [1.0; 4] }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct ParticleInstanceRaw {
    position: [f32; 4],
    color: [f32; 4],
}

impl ToRaw for ParticleInstance {
    fn to_raw(&self) -> Vec<u8> {
        let raw = self.raw();
        bytes_of(&raw).to_vec()
    }
}

impl Descriptor for ParticleInstance {
    fn desc<'a>() -> Option<wgpu::VertexBufferLayout<'a>> {
        use std::mem;
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ParticleInstanceRaw>() as wgpu::BufferAddress,
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
            ],
        })
    }
}