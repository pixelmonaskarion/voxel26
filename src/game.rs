use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};

use bespoke_engine::{binding::{Binding, Descriptor, UniformBinding, simple_layout_entry}, camera::{Camera, CameraRaw}, model::{Render, ToRaw}, resource_loader::{load_resource, load_resource_string}, shader::{Shader, ShaderConfig, ShaderType}, surface_context::SurfaceCtx, texture::Texture, window::{WindowConfig, WindowHandler}};
use bytemuck::{bytes_of, NoUninit};
use cgmath::{Vector2, Vector3, vec3};
use wgpu::{Color, Features, Limits, RenderPass};
use winit::{dpi::PhysicalPosition, event::{KeyEvent, Modifiers, TouchPhase}, keyboard::{KeyCode, PhysicalKey::Code}};
use crate::{RESOURCES, chunk::{CHUNK_SIZE, ChunkManager}, player::Player, util::{self}};

pub struct Game<'a> {
    camera: Camera,
    player: Player,
    screen_size: [f32; 2],
    screen_info_binding: UniformBinding<ScreenInfo>,
    start_time: u128,
    keys_down: Vec<KeyCode>,
    touch_positions: HashMap<u64, PhysicalPosition<f64>>,
    moving_bc_finger: Option<u64>,
    post_processing_shader: Shader<'a>,

    chunk_manager: ChunkManager,
    chunk_shader: Shader<'a>,
    atlas_uniform: UniformBinding<Texture>,
}

#[repr(C)]
#[derive(NoUninit, Copy, Clone)]
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
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
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
        }
    }
}

impl ToRaw for Vertex {
    fn to_raw(&self) -> Vec<u8> {
        bytes_of(self).to_vec()
    }
}


impl <'a> Game<'a> {
    pub async fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let screen_size = [surface_ctx.size().0 as f32, surface_ctx.size().1 as f32];
        let camera = Camera {
            eye: Vector3::new(-1.0, 0.0, 0.0),
            aspect: screen_size[0] / screen_size[1],
            fovy: 70.0,
            znear: 0.1,
            zfar: 10000.0,
            ground: 0.0,
            sky: 0.0,
        };
        let screen_info_binding = UniformBinding::new(surface_ctx.device(), "Screen Info", ScreenInfo::new(screen_size, 0.0, camera.to_raw()), None);
        let mut chunk_manager = ChunkManager::new(surface_ctx);
        chunk_manager.get_chunk_or_create([0; 3]);
        chunk_manager.generate_blocks([0; 3], [0.0; 3]);
        // let low = 0;
        // let high = 4;
        // for cx in low..high {
        //     for cy in low..high {
        //         for cz in low..high {
        //             let chunk = chunk_manager.get_chunk_or_create([cx, cy, cz], surface_ctx);
        //             for x in 0..CHUNK_SIZE {
        //                 for y in 0..CHUNK_SIZE {
        //                     for z in 0..CHUNK_SIZE {
        //                         chunk.set_block(surface_ctx, [x, y, z], index_in_chunk(x, y, z) as u16 +3);
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
        // for cx in low..high {
        //     for cy in low..high {
        //         for cz in low..high {
        //             chunk_manager.generate_cpu([cx, cy, cz], surface_ctx);
        //         }
        //     }
        // }
        let post_processing_shader = Shader::new_post_process("res/shaders/post_process.wgsl", surface_ctx.device(), surface_ctx.config().format, vec![], vec![]);
        let atlas_uniform = UniformBinding::new(surface_ctx.device(), "Atlas", Texture::from_bytes(surface_ctx.device(), surface_ctx.queue(), &load_resource("res/atlas.png").unwrap(), "Atlas", None, None).unwrap(), None);
        let chunk_shader = Shader::new_uniform("res/shaders/chunk.wgsl", surface_ctx.device(), vec![surface_ctx.config().format], vec![&screen_info_binding, &atlas_uniform], vec![Vertex::desc()], ShaderConfig { line_mode: wgpu::PolygonMode::Fill, ..Default::default() });
        Self {
            camera,
            player: Player::new(vec3(0.0, 0.0, 0.0)),
            screen_size,
            screen_info_binding,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            keys_down: vec![],
            touch_positions: HashMap::new(),
            moving_bc_finger: None,
            post_processing_shader,
            chunk_shader,
            chunk_manager,
            atlas_uniform,
        }
    }
}

impl <'s> WindowHandler for Game<'s> {
    fn resize(&mut self, _surface_ctx: &dyn SurfaceCtx, new_size: Vector2<u32>) {
        self.camera.aspect = new_size.x as f32 / new_size.y as f32;
        self.screen_size = [new_size.x as f32, new_size.y as f32];
    }

    fn render<'a: 'b, 'b>(&'a mut self, surface_ctx: &dyn SurfaceCtx, render_pass: &mut RenderPass<'b>, delta: f64) {
        self.update(delta);
        let time = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()-self.start_time) as f32 / 1000.0;
        self.screen_info_binding.set_data(surface_ctx.queue(), ScreenInfo::new(self.screen_size, time, self.camera.to_raw()));

        let mut count = 0;
        for chunk_pos in util::positions(100) {
            let relative_pos = [chunk_pos[0] + (self.camera.eye.x / CHUNK_SIZE as f32).floor() as i32, chunk_pos[1] + (self.camera.eye.y / CHUNK_SIZE as f32).floor() as i32, chunk_pos[2] + (self.camera.eye.z / CHUNK_SIZE as f32).floor() as i32];
            if !self.chunk_manager.chunk_exists(relative_pos) {
                self.chunk_manager.get_chunk_or_create(relative_pos);
                self.chunk_manager.generate_blocks(relative_pos, self.player.position.into());
                count += 1;
                if count == 3 {
                    break;
                }
            }
        }
        self.chunk_manager.poll_channels(self.player.position.into());

        self.chunk_shader.bind(render_pass);
        render_pass.set_bind_group(0, &self.screen_info_binding.binding, &[]);
        render_pass.set_bind_group(1, &self.atlas_uniform.binding, &[]);
        if self.keys_down.contains(&KeyCode::KeyC) {
            for chunk_position in self.chunk_manager.chunk_positions().cloned().collect::<Vec<[i32; 3]>>() {
                self.chunk_manager.generate_model(chunk_position, self.player.position.into());
            }
        }
        for chunk in self.chunk_manager.chunks(self.camera.eye.into()) {
            if chunk.visible() {
                chunk.render(render_pass);
            }
        }
    }

    fn config(&self) -> Option<WindowConfig> {
        Some(WindowConfig { background_color: Some(Color::BLACK), enable_post_processing: Some(false) })
    }

    fn mouse_moved(&mut self, _surface_ctx: &dyn SurfaceCtx, _mouse_pos: PhysicalPosition<f64>) {

    }
    
    fn input_event(&mut self, _surface_ctx: &dyn SurfaceCtx, input_event: &KeyEvent, _modifiers: &Modifiers) {
        if let Code(code) = input_event.physical_key {
            if input_event.state.is_pressed() {
                if !self.keys_down.contains(&code) {
                    self.keys_down.push(code);
                }
            } else {
                if let Some(i) = self.keys_down.iter().position(|x| x == &code) {
                    self.keys_down.remove(i);
                }
            }
        }
    }
    
    fn mouse_motion(&mut self, _surface_ctx: &dyn SurfaceCtx, delta: (f64, f64)) {
        self.camera.ground += (delta.0 / 500.0) as f32;
        self.camera.sky -= (delta.1 / 500.0) as f32;
        self.camera.sky = self.camera.sky.clamp(std::f32::consts::PI*-0.499, std::f32::consts::PI*0.499);
    }
    
    fn touch(&mut self, surface_ctx: &dyn SurfaceCtx, touch: &winit::event::Touch) {
        match touch.phase {
            TouchPhase::Moved => {
                if let Some(last_position) = self.touch_positions.get(&touch.id) {
                    let delta = (touch.location.x-last_position.x, touch.location.y-last_position.y);
                    self.mouse_motion(surface_ctx, delta);
                    self.touch_positions.insert(touch.id, touch.location);
                }
            }
            TouchPhase::Started => {
                if touch.location.x <= self.screen_size[0] as f64 / 2.0 {
                    self.touch_positions.insert(touch.id, touch.location);
                } else {
                    self.moving_bc_finger = Some(touch.id);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touch_positions.remove(&touch.id);
                if self.moving_bc_finger == Some(touch.id) {
                    self.moving_bc_finger = None;
                }
            }
        }
    }
    
    fn post_process_render<'a: 'b, 'c: 'b, 'b>(&'a mut self, surface_ctx: &'c dyn SurfaceCtx, render_pass: & mut RenderPass<'b>, _surface_texture: &'c UniformBinding<Texture>) {
        self.post_processing_shader.bind(render_pass);
        surface_ctx.screen_model().render(render_pass);
    }
    
    fn limits() -> wgpu::Limits {
        Limits {
            max_bind_groups: 7,
            max_texture_dimension_2d: 8976,
            ..Default::default()
        }
    }
    
    fn other_window_event(&mut self, _surface_context: &dyn SurfaceCtx, _event: &winit::event::WindowEvent) {
        
    }

    fn required_features() -> wgpu::Features {
        Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | Features::POLYGON_MODE_LINE
    }

    fn surface_config() -> Option<bespoke_engine::window::SurfaceConfig> {
        None
    }

    fn custom_shader_type_source() -> String {
        load_resource_string("res/shaders/custom_shader_types.wgsl").into()
    }

    fn resources() -> Option<&'static phf::Map<&'static str, bespoke_engine::resource_loader::ResourceType>> {
        Some(&RESOURCES)
    }
}

impl <'s> Game<'s> {
    fn update(&mut self, delta: f64) {
        let speed = 100.0 * delta as f32;
        if self.keys_down.contains(&KeyCode::KeyW) || self.moving_bc_finger.is_some() {
            self.player.velocity += self.camera.get_walking_vec() * speed;
        }
        if self.moving_bc_finger.is_some() {
            self.player.velocity += self.camera.get_forward_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::KeyS) {
            self.player.velocity -= self.camera.get_walking_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::KeyA) {
            self.player.velocity -= self.camera.get_right_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::KeyD) {
            self.player.velocity += self.camera.get_right_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::Space) {
            if self.player.time_since_ground < 0.1 {
                self.player.velocity += Vector3::unit_y() * 7.0;
                self.player.time_since_ground = 1.0;
            }
        }
        if self.player.movement_move == 1 {
            if self.keys_down.contains(&KeyCode::ShiftLeft) {
                self.player.velocity -= Vector3::unit_y() * speed;
            }
            if self.keys_down.contains(&KeyCode::Space) {
                self.player.velocity += Vector3::unit_y() * speed;
            }
        }
        if self.player.movement_move == 0 {
            self.player.velocity.y -= 20.0 * delta as f32;
        }
        self.player.velocity.x *= 0.9;
        self.player.velocity.z *= 0.9;
        if self.player.movement_move == 1 {
            self.player.velocity.y *= 0.9;
        }
        self.player.time_since_ground += delta;
        self.player.move_player(self.player.velocity * delta as f32, &self.chunk_manager);
        self.camera.eye = self.player.position;

        if self.keys_down.contains(&KeyCode::KeyC) {
            self.player.movement_move = 1;
        }
        if self.keys_down.contains(&KeyCode::KeyV) {
            self.player.movement_move = 0;
        }
    }
}

#[derive(NoUninit, Clone, Copy)]
#[repr(C)]
pub struct ScreenInfo {
    screen_size: [f32; 2],
    time: f32,
    padding: f32,
    camera_raw: CameraRaw,
}

impl ScreenInfo {
    pub fn new(screen_size: [f32; 2], time: f32, camera_raw: CameraRaw) -> Self {
        Self {
            screen_size,
            time,
            padding: 0.0,
            camera_raw,
        }
    }
}

impl Binding for ScreenInfo {
    fn create_resources<'a>(&'_ self) -> Vec<bespoke_engine::binding::Resource<'_>> {
        vec![bespoke_engine::binding::Resource::Simple(bytes_of(self).to_vec())]
    }

    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![simple_layout_entry(0)]
    }

    fn shader_type() -> bespoke_engine::shader::ShaderType {
        ShaderType {
            var_types: vec!["<uniform>".into()],
            wgsl_types: vec!["ScreenInfo".into()]
        }    
    }
}