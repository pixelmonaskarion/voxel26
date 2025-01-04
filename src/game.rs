use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};

use bespoke_engine::{binding::{simple_layout_entry, Binding, Descriptor, UniformBinding}, camera::{Camera, CameraRaw}, model::{Render, ToRaw}, shader::{Shader, ShaderType}, surface_context::SurfaceCtx, texture::Texture, window::{WindowConfig, WindowHandler}};
use bytemuck::{bytes_of, NoUninit};
use cgmath::{Vector2, Vector3};
use wgpu::{Color, Features, Limits, RenderPass};
use winit::{dpi::PhysicalPosition, event::{KeyEvent, TouchPhase}, keyboard::{KeyCode, PhysicalKey::Code}};

pub struct Game {
    camera: Camera,
    screen_size: [f32; 2],
    screen_info_binding: UniformBinding<ScreenInfo>,
    start_time: u128,
    keys_down: Vec<KeyCode>,
    touch_positions: HashMap<u64, PhysicalPosition<f64>>,
    moving_bc_finger: Option<u64>,
    post_processing_shader: Shader,
}

#[repr(C)]
#[derive(NoUninit, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_pos: [f32; 2],
    pub normal: [f32; 3],
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
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
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


impl Game {
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let screen_size = [surface_ctx.size().0 as f32, surface_ctx.size().1 as f32];
        let camera = Camera {
            eye: Vector3::new(1.0, 0.0, 0.0),
            aspect: screen_size[0] / screen_size[1],
            fovy: 70.0,
            znear: 0.1,
            zfar: 100.0,
            ground: 0.0,
            sky: 0.0,
        };
        let screen_info_binding = UniformBinding::new(surface_ctx.device(), "Screen Info", ScreenInfo::new(screen_size, 0.0, camera.to_raw()), None);

        let post_processing_shader = Shader::new_post_process(include_str!("shaders/post_process.wgsl"), surface_ctx.device(), surface_ctx.config().format, vec![], vec![]);
        Self {
            camera,
            screen_size,
            screen_info_binding,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            keys_down: vec![],
            touch_positions: HashMap::new(),
            moving_bc_finger: None,
            post_processing_shader,
        }
    }
}

impl WindowHandler for Game {
    fn resize(&mut self, _surface_ctx: &dyn SurfaceCtx, new_size: Vector2<u32>) {
        self.camera.aspect = new_size.x as f32 / new_size.y as f32;
        self.screen_size = [new_size.x as f32, new_size.y as f32];
    }

    fn render<'a: 'b, 'b>(&'a mut self, surface_ctx: &dyn SurfaceCtx, _render_pass: & mut RenderPass<'b>, delta: f64) {
        self.update(delta);
        let time = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()-self.start_time) as f32 / 1000.0;
        self.screen_info_binding.set_data(&surface_ctx.device(), ScreenInfo::new(self.screen_size, time, self.camera.to_raw()));
    }

    fn config(&self) -> Option<WindowConfig> {
        Some(WindowConfig { background_color: Some(Color::BLACK), enable_post_processing: Some(true) })
    }

    fn mouse_moved(&mut self, _surface_ctx: &dyn SurfaceCtx, _mouse_pos: PhysicalPosition<f64>) {

    }
    
    fn input_event(&mut self, _surface_ctx: &dyn SurfaceCtx, input_event: &KeyEvent) {
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
        Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
    }

    fn surface_config() -> Option<bespoke_engine::window::SurfaceConfig> {
        None
    }

    fn custom_shader_type_source() -> String {
        include_str!("shaders/custom_shader_types.wgsl").into()
    }
}

impl Game {
    fn update(&mut self, delta: f64) {
        let speed = 0.005 * delta as f32;
        if self.keys_down.contains(&KeyCode::KeyW) || self.moving_bc_finger.is_some() {
            self.camera.eye += self.camera.get_walking_vec() * speed;
        }
        if self.moving_bc_finger.is_some() {
            self.camera.eye += self.camera.get_forward_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::KeyS) {
            self.camera.eye -= self.camera.get_walking_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::KeyA) {
            self.camera.eye -= self.camera.get_right_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::KeyD) {
            self.camera.eye += self.camera.get_right_vec() * speed;
        }
        if self.keys_down.contains(&KeyCode::Space) {
            self.camera.eye += Vector3::unit_y() * speed;
        }
        if self.keys_down.contains(&KeyCode::ShiftLeft) {
            self.camera.eye -= Vector3::unit_y() * speed;
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
    fn create_resources<'a>(&'a self) -> Vec<bespoke_engine::binding::Resource> {
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