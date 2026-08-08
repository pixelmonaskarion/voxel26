use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};

use bespoke_engine::{binding::{Binding, Descriptor, UniformBinding, create_layout, simple_layout_entry}, camera::{Camera, CameraRaw, OrthographicCamera}, culling::AABB, model::{Model, Render, ToRaw}, resource_loader::{load_resource, load_resource_string}, shader::{Shader, ShaderConfig, ShaderType}, surface_context::SurfaceCtx, texture::{DepthTexture, Texture}, window::{BasicVertex, WindowConfig, WindowHandler}};
use bytemuck::{bytes_of, NoUninit};
use cgmath::{InnerSpace, Vector2, Vector3, Zero, vec2, vec3};
use wgpu::{Color, Features, Limits, RenderPass, RenderPassDepthStencilAttachment, TextureFormat};
use winit::{dpi::PhysicalPosition, event::{KeyEvent, Modifiers, MouseButton, TouchPhase, WindowEvent}, keyboard::{KeyCode, PhysicalKey::Code}};
use crate::{RESOURCES, block_models::block_model, blocks::{AIR, Block, NOT_RENDERED_LAYER, get_block}, chunk::{CHUNK_SIZE, ChunkManager}, cube_outline::{cube_outline_model, cube_outline_shader}, entity::{Entity, EntityRenderManager, EntityType, TypedEntity}, inventory::{InventoryItemStack, Item, ItemAtlas, ItemStack}, particles::{Particle, ParticleManager, ParticleType}, player::Player, ui::{generate_crosshair_ui_model, generate_hotbar_background_ui_models, generate_hotbar_item_ui_models}, util::{self, chunk_for_block_position, chunk_for_world_position}};

pub struct Game<'a> {
    camera: Camera,
    player: Player,
    screen_size: [f32; 2],
    screen_info_binding: UniformBinding<ScreenInfo>,
    start_time: u128,
    keys_down: Vec<KeyCode>,
    mouse_down: Vec<MouseButton>,
    touch_positions: HashMap<u64, PhysicalPosition<f64>>,
    moving_bc_finger: Option<u64>,
    post_processing_shader: Shader<'a>,

    particle_manager: ParticleManager<'a>,

    chunk_manager: ChunkManager,
    chunk_shader: Shader<'a>,
    atlas_uniform: UniformBinding<Texture>,
    entity_render_manager: EntityRenderManager<'a>,
    item_atlas: ItemAtlas<'a>,

    cube_outline_shader: Shader<'a>,
    cube_outline_model: Model,
    block_renderer_shader: Shader<'a>,
    block_renderer_screen_info: UniformBinding<ScreenInfo>,
    hotbar_item_ui_models: Vec<Model>,
    hotbar_background_ui_models: Vec<Model>,
    crosshair_model: Model,
    ui_shader: Shader<'a>,
    atlas_subsection_uniform: UniformBinding<[f32; 4]>,
    item_ui_shader: Shader<'a>,
    ui_texture_uniform: UniformBinding<Texture>,
    block_render_depth_texture: DepthTexture,

    temp_model: Model,
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
        let post_processing_shader = Shader::new_post_process("res/shaders/post_process.wgsl", surface_ctx.device(), surface_ctx.config().format, vec![&create_layout::<Texture>(surface_ctx.device())], vec![&Texture::shader_type()]);
        let atlas_uniform = UniformBinding::new(surface_ctx.device(), "Atlas", Texture::from_bytes(surface_ctx.device(), surface_ctx.queue(), &load_resource("res/atlas.png").unwrap(), "Atlas", None, None).unwrap(), None);
        let chunk_shader = Shader::new_uniform("res/shaders/chunk.wgsl", surface_ctx.device(), vec![surface_ctx.config().format], vec![&screen_info_binding, &atlas_uniform], vec![Vertex::desc()], ShaderConfig { line_mode: wgpu::PolygonMode::Fill, ..Default::default() });

        let cube_outline_shader = cube_outline_shader(surface_ctx.device(), vec![surface_ctx.config().format], &screen_info_binding);
        let cube_outline_model = Model::new_empty::<u32>(AABB::zero(), surface_ctx.device());

        let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
        let hotbar_item_ui_models = generate_hotbar_item_ui_models(surface_ctx, aspect_ratio);
        let hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, aspect_ratio, 0);
        let ui_texture_uniform = UniformBinding::new(surface_ctx.device(), "UI Textures", Texture::from_bytes(surface_ctx.device(), surface_ctx.queue(), &load_resource("res/ui.png").unwrap(), "UI", None, None).unwrap(), None);
        let ui_shader = Shader::new_post_process("res/shaders/ui.wgsl", surface_ctx.device(), surface_ctx.config().format, vec![&create_layout::<Texture>(surface_ctx.device())], vec![&Texture::shader_type()]);
        let atlas_subsection_uniform = UniformBinding::new(surface_ctx.device(), "Atlas Subsection", [0.0; 4], None);
        let item_ui_shader = Shader::new_post_process("res/shaders/item_ui.wgsl", surface_ctx.device(), surface_ctx.config().format, vec![&create_layout::<Texture>(surface_ctx.device()), &atlas_subsection_uniform.layout], vec![&Texture::shader_type(), &atlas_subsection_uniform.shader_type]);
        let block_renderer_shader = Shader::new_uniform("res/shaders/chunk.wgsl", surface_ctx.device(), vec![surface_ctx.config().format], vec![&screen_info_binding, &atlas_uniform], vec![Vertex::desc()], ShaderConfig { line_mode: wgpu::PolygonMode::Fill, ..Default::default() });
        let block_renderer_screen_info = UniformBinding::new(surface_ctx.device(), "", ScreenInfo { 
            camera_raw: OrthographicCamera {
                eye: vec3(1.0, 1.0, 1.0),
                target: vec3(0.5, 0.5, 0.5),
                left: -1.0,
                right: 1.0,
                bottom: -1.0,
                top: 1.0,
                far: 0.0,
                near: 3.00,
            }.to_raw(),
            time: 0.0,
            screen_size: [64.0, 64.0],
            padding: 0.0,
         }, None);
        let block_render_depth_texture = DepthTexture::create_depth_texture(surface_ctx.device(), 64, 64, "Block Renderer Depth Texture");

        let crosshair_model = generate_crosshair_ui_model(surface_ctx, aspect_ratio);

        let particle_manager = ParticleManager::new(surface_ctx, &screen_info_binding);
        let entity_render_manager = EntityRenderManager::new(surface_ctx);
        let mut item_atlas = ItemAtlas::new(surface_ctx, &atlas_uniform);
        Self {
            camera,
            player: Player::new(vec3(0.0, 10.0, 0.0), &mut item_atlas, &atlas_uniform, surface_ctx),
            screen_size,
            screen_info_binding,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            keys_down: vec![],
            mouse_down: vec![],
            touch_positions: HashMap::new(),
            moving_bc_finger: None,
            post_processing_shader,
            chunk_shader,
            chunk_manager,
            atlas_uniform,
            cube_outline_shader,
            cube_outline_model,
            hotbar_item_ui_models,
            hotbar_background_ui_models,
            ui_shader,
            ui_texture_uniform,
            block_renderer_shader,
            block_renderer_screen_info,
            temp_model: Model::new_empty::<u32>(AABB::zero(), surface_ctx.device()),
            block_render_depth_texture,
            crosshair_model,
            particle_manager,
            entity_render_manager,
            item_atlas,
            atlas_subsection_uniform,
            item_ui_shader,
        }
    }
}

impl <'s> WindowHandler for Game<'s> {
    fn resize(&mut self, surface_ctx: &dyn SurfaceCtx, new_size: Vector2<u32>) {
        let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
        self.camera.aspect = aspect_ratio;
        self.screen_size = [new_size.x as f32, new_size.y as f32];
        self.hotbar_item_ui_models = generate_hotbar_item_ui_models(surface_ctx, aspect_ratio);
        self.crosshair_model = generate_crosshair_ui_model(surface_ctx, aspect_ratio);
        self.hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, aspect_ratio, self.player.inventory.selected);
    }

    fn render<'a: 'b, 'b>(&'a mut self, surface_ctx: &'b dyn SurfaceCtx, render_pass: &mut RenderPass<'b>, delta: f64) {
        self.update(surface_ctx, delta);
        self.particle_manager.run_step(surface_ctx);
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
        // if self.keys_down.contains(&KeyCode::KeyC) {
        //     for chunk_position in self.chunk_manager.chunk_positions().cloned().collect::<Vec<[i32; 3]>>() {
        //         self.chunk_manager.generate_model(chunk_position, self.player.position.into());
        //     }
        // }
        for chunk in self.chunk_manager.chunks_sorted(self.camera.eye.into()) {
            if chunk.visible() {
                //TODO: move camera stuff out of particle manager
                chunk.render(render_pass, &self.entity_render_manager, &self.item_atlas, &self.chunk_shader, &self.particle_manager.camera_view_binding, &self.particle_manager.camera_projection_binding, &self.atlas_uniform, surface_ctx);
            }
        }

        if let Some((target_position, _)) = self.player.raycast(self.camera.get_forward_vec(), 6.0, &self.chunk_manager) {
            self.cube_outline_shader.bind(render_pass);
            self.cube_outline_model = cube_outline_model(surface_ctx.device(), self.camera.build_view_projection_matrix(), Vector3::<i32>::from(target_position).cast().unwrap());
            self.cube_outline_model.render(render_pass);
        }

        self.particle_manager.render(&self.screen_info_binding, &self.camera, render_pass, surface_ctx);
    }

    fn config(&self) -> Option<WindowConfig> {
        Some(WindowConfig { background_color: Some(Color { r: 36.0/255.0, g: 105.0/255.0, b: 245.0/255.0, a: 1.0}), enable_post_processing: Some(true) })
    }

    fn mouse_moved(&mut self, _surface_ctx: &dyn SurfaceCtx, _mouse_pos: PhysicalPosition<f64>) {

    }

    fn mouse_input(&mut self, _surface_context: &dyn SurfaceCtx, element_state: &winit::event::ElementState, mouse_button: &winit::event::MouseButton) {
        if element_state.is_pressed() {
            if !self.mouse_down.contains(mouse_button) {
                self.mouse_down.push(*mouse_button);
            }
        } else {
            if let Some(i) = self.mouse_down.iter().position(|x| x == mouse_button) {
                self.mouse_down.remove(i);
            }
        }
    }
    
    fn input_event(&mut self, surface_ctx: &dyn SurfaceCtx, input_event: &KeyEvent, _modifiers: &Modifiers) {
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
        if let Code(KeyCode::Escape) = input_event.physical_key {
            let _ = surface_ctx.window().set_cursor_grab(winit::window::CursorGrabMode::None);
            let _ = surface_ctx.window().set_cursor_visible(true);
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
    
    fn post_process_render<'a: 'b, 'c: 'b, 'b>(&'a mut self, surface_ctx: &'c dyn SurfaceCtx, render_pass: & mut RenderPass<'b>, surface_texture: &'c UniformBinding<Texture>) {
        self.post_processing_shader.bind(render_pass);
        render_pass.set_bind_group(0, &surface_texture.binding, &[]);
        surface_ctx.screen_model().render(render_pass);

        for (i, ui_model) in self.hotbar_item_ui_models.iter().enumerate() {
            let item = &mut self.player.inventory.items[i];
            self.ui_shader.bind(render_pass);
            render_pass.set_bind_group(0, &self.ui_texture_uniform.binding, &[]);
            self.hotbar_background_ui_models[i].render(render_pass);
            self.item_ui_shader.bind(render_pass);
            self.atlas_subsection_uniform.replace_data(surface_ctx.device(), self.item_atlas.subsection_for_position(item.atlas_coordinates));
            render_pass.set_bind_group(0, &self.item_atlas.texture.binding, &[]);
            render_pass.set_bind_group(1, &self.atlas_subsection_uniform.binding, &[]);
            ui_model.render(render_pass);
        }
        self.ui_shader.bind(render_pass);
        render_pass.set_bind_group(0, &self.ui_texture_uniform.binding, &[]);
        self.crosshair_model.render(render_pass);
    }
    
    fn limits() -> wgpu::Limits {
        Limits {
            max_texture_dimension_2d: 8976,
            ..Default::default()
        }
    }
    
    fn other_window_event(&mut self, surface_context: &dyn SurfaceCtx, event: &winit::event::WindowEvent) {
        match event {
            WindowEvent::CursorEntered { .. } => {
                if surface_context.window().has_focus() {
                    let _ = surface_context.window().set_cursor_grab(winit::window::CursorGrabMode::Locked);
                    surface_context.window().set_cursor_visible(false);
                }
            },
            WindowEvent::Focused(focused) => {
                if *focused {
                    let _ = surface_context.window().set_cursor_grab(winit::window::CursorGrabMode::Locked);
                    surface_context.window().set_cursor_visible(false);
                } else {
                    let _ = surface_context.window().set_cursor_grab(winit::window::CursorGrabMode::None);
                    surface_context.window().set_cursor_visible(true);
                }
            }
            _ => {}
        }
    }

    fn required_features() -> wgpu::Features {
        Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | Features::POLYGON_MODE_LINE | Features::VERTEX_WRITABLE_STORAGE
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
    fn update(&mut self, surface_ctx: &dyn SurfaceCtx, delta: f64) {
        let speed = 100.0 * delta as f32;
        let mut movement = vec3(0.0, 0.0, 0.0);
        if self.keys_down.contains(&KeyCode::KeyW) || self.moving_bc_finger.is_some() {
            movement += self.camera.get_walking_vec();
        }
        if self.moving_bc_finger.is_some() {
            movement += self.camera.get_walking_vec();
        }
        if self.keys_down.contains(&KeyCode::KeyS) {
            movement -= self.camera.get_walking_vec();
        }
        if self.keys_down.contains(&KeyCode::KeyA) {
            movement -= self.camera.get_right_vec();
        }
        if self.keys_down.contains(&KeyCode::KeyD) {
            movement += self.camera.get_right_vec();
        }
        if self.keys_down.contains(&KeyCode::Space) {
            if self.player.time_since_ground < 0.1 {
                self.player.velocity += Vector3::unit_y() * 7.0;
                self.player.time_since_ground = 1.0;
            }
        }
        if self.player.movement_mode == 1 {
            if self.keys_down.contains(&KeyCode::ShiftLeft) {
                self.player.velocity -= Vector3::unit_y() * speed;
            }
            if self.keys_down.contains(&KeyCode::Space) {
                self.player.velocity += Vector3::unit_y() * speed;
            }
        }
        if !movement.is_zero() {
            self.player.velocity += movement.normalize() * speed;
        }
        if self.player.movement_mode == 0 {
            self.player.velocity.y -= 20.0 * delta as f32;
        }
        self.player.velocity.x *= 0.9;
        self.player.velocity.z *= 0.9;
        if self.player.movement_mode == 1 {
            self.player.velocity.y *= 0.9;
        }
        self.player.time_since_ground += delta;
        self.player.move_player(self.player.velocity * delta as f32, &self.chunk_manager);
        self.camera.eye = self.player.position;

        if self.keys_down.contains(&KeyCode::KeyC) {
            self.player.movement_mode = 1;
        }
        if self.keys_down.contains(&KeyCode::KeyV) {
            self.player.movement_mode = 0;
        }

        if self.mouse_down.contains(&MouseButton::Right) {
            if self.player.break_cooldown == 0 {
                if let Some((coordinate, face)) = self.player.raycast(self.camera.get_forward_vec(), 6.0, &self.chunk_manager) {
                    #[allow(irrefutable_let_patterns)]
                    if let Item::Block(block) = self.player.inventory.selected_item().stack.item && block != AIR {
                        let coordinate = (Vector3::from(coordinate)+face.direction()).into();
                        let before = self.chunk_manager.get_block(coordinate);
                        self.chunk_manager.set_block(coordinate, block.id);
                        if self.player.colliding_world(&self.chunk_manager) {
                            self.chunk_manager.set_block(coordinate, before);
                        }
                        self.chunk_manager.generate_model_and_surroundings(chunk_for_block_position(coordinate), self.player.position.into());
                        self.player.break_cooldown = 8;
                    }
                }
            }
        }
        if self.mouse_down.contains(&MouseButton::Left) {
            if self.player.break_cooldown == 0 {
                if let Some((coordinate, _)) = self.player.raycast(self.camera.get_forward_vec(), 6.0, &self.chunk_manager) {
                    let before = self.chunk_manager.get_block(coordinate);
                    self.chunk_manager.set_block(coordinate, AIR.id);
                    let before_block = get_block(before);
                    for _ in 0..10 {
                        self.particle_manager.add_particle(Particle {
                            particle_type: ParticleType::BlockBreak,
                            position: (Vector3::from(coordinate).cast().unwrap()+vec3(rand::random_range(0.0..1.0), rand::random_range(0.0..1.0), rand::random_range(0.0..1.0))).extend(1.0).into(),
                            velocity: (vec3(rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0))*0.07).into(),
                            color: before_block.color,
                            lifetime: 20,
                        });
                    }
                    self.chunk_manager.get_chunk_or_create(chunk_for_block_position(coordinate)).add_entity(Entity {
                        entity_type: TypedEntity::Item { stack: InventoryItemStack::new(ItemStack::new(Item::Block(before_block), 1), &mut self.item_atlas, &self.atlas_uniform, surface_ctx) },
                        position: Vector3::<i32>::from(coordinate).cast().unwrap()+vec3(0.5, 0.5, 0.5),
                        velocity: (vec3(rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0))*0.07).into(),
                    });
                    self.chunk_manager.generate_model_and_surroundings(chunk_for_block_position(coordinate), self.player.position.into());
                    self.player.break_cooldown = 8;
                }
            }
        }
        self.player.break_cooldown = (self.player.break_cooldown-1).max(0);

        let mut any = false;
        for (i, key) in [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
        ].iter().enumerate() {
            if self.keys_down.contains(&key) {
                self.player.inventory.selected = i;
                any = true;
            }
        }
        if any {
            self.hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, self.camera.aspect, self.player.inventory.selected);
        }

        let mut move_entities = vec![];
        for pos in self.chunk_manager.chunk_positions_owned() {
            for entity_type in self.chunk_manager.get_chunk_or_create(pos).entities.keys().cloned().collect::<Vec<EntityType>>() {
                let mut i = 0;
                let mut len = self.chunk_manager.get_chunk_or_create(pos).entities.get(&entity_type).unwrap().len();
                let mut temp_entity = Entity { position: vec3(0.0, 0.0, 0.0), velocity: vec3(0.0, 0.0, 0.0), entity_type: TypedEntity::Marker };
                while i < len {
                    std::mem::swap(&mut self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap()[i], &mut temp_entity);
                    temp_entity.update(&self.chunk_manager, delta);
                    std::mem::swap(&mut self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap()[i], &mut temp_entity);
                    if chunk_for_world_position(self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap()[i].position.into()) != pos {
                        move_entities.push(self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap().remove(i));
                    } else {
                        i += 1;
                    }
                    len = self.chunk_manager.get_chunk_or_create(pos).entities.get(&entity_type).unwrap().len();
                }
            }
        }
        while let Some(entity) = move_entities.pop() {
            self.chunk_manager.get_chunk_or_create(chunk_for_world_position(entity.position.into())).add_entity(entity);
        }
    }

    pub fn render_block(&self, surface_ctx: &dyn SurfaceCtx, width: u32, height: u32, format: TextureFormat, block: Block) -> Texture {
        let render_texture = Texture::blank_texture(surface_ctx.device(), width, height, format);
        if block.layer == NOT_RENDERED_LAYER {
            return render_texture;
        }
        let mut encoder = surface_ctx.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let block_model = block_model(surface_ctx, block);
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_texture.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.block_render_depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
            });
            self.block_renderer_shader.bind(&mut render_pass);
            render_pass.set_bind_group(0, &self.block_renderer_screen_info.binding, &[]);
            render_pass.set_bind_group(1, &self.atlas_uniform.binding, &[]);
            block_model.render(&mut render_pass);
        }
        surface_ctx.queue().submit([encoder.finish()]);
        render_texture
    }
}

#[derive(NoUninit, Clone, Copy)]
#[repr(C)]
pub struct ScreenInfo {
    pub screen_size: [f32; 2],
    pub time: f32,
    pub padding: f32,
    pub camera_raw: CameraRaw,
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