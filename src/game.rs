use std::{collections::HashMap, time::{Duration, SystemTime, UNIX_EPOCH}};

use bespoke_engine::{binding::{Binding, Descriptor, DynamicOffsetUniform, DynamicOffsetUniformVec, UniformBinding, create_layout, simple_layout_entry}, camera::{Camera, CameraRaw}, model::{Model, Render, ToRaw}, resource_loader::{load_resource, load_resource_string}, shader::{Shader, ShaderConfig, ShaderType}, surface_context::SurfaceCtx, texture::Texture, window::{WindowConfig, WindowHandler}};
use bytemuck::{bytes_of, NoUninit};
use cgmath::{InnerSpace, MetricSpace, Vector2, Vector3, Zero, vec3};
use wgpu::{Color, Features, Limits, RenderPass};
use wgpu_text::{BrushBuilder, TextBrush, glyph_brush::{Layout, OwnedSection, OwnedText, ab_glyph::FontVec}};
use winit::{dpi::PhysicalPosition, event::{KeyEvent, Modifiers, MouseButton, TouchPhase, WindowEvent}, keyboard::{KeyCode, PhysicalKey::Code}};
use crate::{RESOURCES, blocks::{AIR, get_block}, chunk::{CHUNK_SIZE, ChunkManager}, cube_outline::{cube_outline_model, cube_outline_shader}, entity::{Entity, EntityRenderManager, EntityType, TypedEntity}, inventory::{InventoryItemStack, Item, ItemAtlas, ItemStack}, particles::{Particle, ParticleManager, ParticleType}, player::Player, ui::{InventoryModel, OpenInventory, UIVertex, create_inventory_model, generate_crosshair_ui_model, generate_health_ui_models, generate_hotbar_background_ui_models, generate_hotbar_item_ui_models, hotbar_item_height, text_sections_for_inventory}, util::{self, chunk_for_block_position, chunk_for_world_position}};

pub struct Game<'a> {
    camera: Camera,
    player: Player,
    screen_size: [f32; 2],
    screen_info_binding: UniformBinding<ScreenInfo>,
    start_time: u128,
    keys_down: Vec<KeyCode>,
    new_keys_down: Vec<KeyCode>,
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
    cube_outline_model: Option<Model>,
    hotbar_item_ui_models: Vec<Model>,
    hotbar_background_ui_models: Vec<Model>,
    health_ui_models: Vec<Model>,
    crosshair_model: Model,
    ui_shader: Shader<'a>,
    hotbar_atlas_subsections_uniform: UniformBinding<DynamicOffsetUniform<[f32; 4], 9>>,
    item_ui_shader: Shader<'a>,
    ui_texture_uniform: UniformBinding<Texture>,
    text_brush: TextBrush<FontVec>,
    open_inventory: Option<OpenInventory>,
    open_inventory_model: Option<InventoryModel>,
    open_inventory_item_subsections_uniform: Option<UniformBinding<DynamicOffsetUniformVec<[f32; 4]>>>,
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

        let post_processing_shader = Shader::new_post_process("res/shaders/post_process.wgsl", surface_ctx.device(), surface_ctx.config().format, vec![&create_layout::<Texture>(surface_ctx.device())], vec![&Texture::shader_type()]);
        let atlas_uniform = UniformBinding::new(surface_ctx.device(), "Atlas", Texture::from_bytes(surface_ctx.device(), surface_ctx.queue(), &load_resource("res/atlas.png").unwrap(), "Atlas", None, None).unwrap(), None);
        let chunk_shader = Shader::new_uniform("res/shaders/chunk.wgsl", surface_ctx.device(), vec![surface_ctx.config().format], vec![&screen_info_binding, &atlas_uniform], vec![Vertex::desc()], ShaderConfig { line_mode: wgpu::PolygonMode::Fill, ..Default::default() });

        let cube_outline_shader = cube_outline_shader(surface_ctx.device(), vec![surface_ctx.config().format], &screen_info_binding);
        let cube_outline_model = None;

        let hotbar_item_ui_models = generate_hotbar_item_ui_models(surface_ctx);
        let hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, 0);
        let health_ui_models = generate_health_ui_models(surface_ctx, 20.0);
        let ui_texture_uniform = UniformBinding::new(surface_ctx.device(), "UI Textures", Texture::from_bytes(surface_ctx.device(), surface_ctx.queue(), &load_resource("res/ui.png").unwrap(), "UI", None, None).unwrap(), None);
        let ui_shader = Shader::new("res/shaders/ui.wgsl", surface_ctx.device(), vec![surface_ctx.config().format], vec![&create_layout::<Texture>(surface_ctx.device())], vec![&Texture::shader_type()], vec![UIVertex::desc()], ShaderConfig { enable_depth_texture: false, ..Default::default() });
        let hotbar_atlas_subsections_uniform = UniformBinding::new(surface_ctx.device(), "Atlas Subsection", DynamicOffsetUniform { values: [[0.0; 4]; 9], alignment: surface_ctx.device().limits().min_uniform_buffer_offset_alignment as usize }, None);
        let item_ui_shader = Shader::new("res/shaders/item_ui.wgsl", surface_ctx.device(), vec![surface_ctx.config().format], vec![&create_layout::<Texture>(surface_ctx.device()), &hotbar_atlas_subsections_uniform.layout], vec![&Texture::shader_type(), &hotbar_atlas_subsections_uniform.shader_type], vec![UIVertex::desc()], ShaderConfig { enable_depth_texture: false, ..Default::default() });

        let crosshair_model = generate_crosshair_ui_model(surface_ctx);

        let particle_manager = ParticleManager::new(surface_ctx, &screen_info_binding);
        let entity_render_manager = EntityRenderManager::new(surface_ctx);
        let mut item_atlas = ItemAtlas::new(surface_ctx, &atlas_uniform);
        let text_brush = BrushBuilder::using_font(FontVec::try_from_vec(load_resource("res/unifont.ttf").unwrap()).unwrap()).build(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, surface_ctx.config().format);
        Self {
            camera,
            player: Player::new(vec3(0.0, 10.0, 0.0), &mut item_atlas, &atlas_uniform, surface_ctx),
            screen_size,
            screen_info_binding,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            keys_down: vec![],
            new_keys_down: vec![],
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
            health_ui_models,
            ui_shader,
            ui_texture_uniform,
            crosshair_model,
            particle_manager,
            entity_render_manager,
            item_atlas,
            hotbar_atlas_subsections_uniform,
            item_ui_shader,
            text_brush,
            open_inventory: None,
            open_inventory_model: None,
            open_inventory_item_subsections_uniform: None,
        }
    }
}

impl <'s> WindowHandler for Game<'s> {
    fn resize(&mut self, surface_ctx: &dyn SurfaceCtx, new_size: Vector2<u32>) {
        let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
        self.camera.aspect = aspect_ratio;
        self.screen_size = [new_size.x as f32, new_size.y as f32];
        self.hotbar_item_ui_models = generate_hotbar_item_ui_models(surface_ctx);
        self.crosshair_model = generate_crosshair_ui_model(surface_ctx);
        self.hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, self.player.inventory.selected);
        self.text_brush.resize_view(surface_ctx.config().width as f32, surface_ctx.config().height as f32, surface_ctx.queue());
    }

    fn update(&mut self, surface_ctx: &dyn SurfaceCtx, delta: Duration) {
        let speed = 100.0 * delta.as_secs_f32();
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
            if self.player.time_since_ground < Duration::from_secs_f32(0.1) {
                self.player.velocity += Vector3::unit_y() * 7.0;
                self.player.time_since_ground = Duration::from_secs_f32(1.0);
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
            self.player.velocity.y -= 20.0 * delta.as_secs_f32();
        }
        self.player.velocity.x *= 0.9;
        self.player.velocity.z *= 0.9;
        if self.player.movement_mode == 1 {
            self.player.velocity.y *= 0.9;
        }
        self.player.time_since_ground += delta;
        self.player.move_player(self.player.velocity * delta.as_secs_f32(), &self.chunk_manager);
        self.camera.eye = self.player.position;

        if self.keys_down.contains(&KeyCode::KeyC) {
            self.player.movement_mode = 1;
        }
        if self.keys_down.contains(&KeyCode::KeyV) {
            self.player.movement_mode = 0;
        }
        if self.open_inventory.is_none() {
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
                            self.player.break_cooldown = 16;
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
                        self.player.damage(1.0);
                        self.chunk_manager.get_chunk_or_create(chunk_for_block_position(coordinate)).add_entity(Entity {
                            entity_type: TypedEntity::Item { stack: InventoryItemStack::new(ItemStack::new(Item::Block(before_block), 1), &mut self.item_atlas, &self.atlas_uniform, surface_ctx) },
                            position: Vector3::<i32>::from(coordinate).cast().unwrap()+vec3(0.5, 0.5, 0.5),
                            velocity: (vec3(rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0))*5.0).into(),
                        });
                        self.chunk_manager.generate_model_and_surroundings(chunk_for_block_position(coordinate), self.player.position.into());
                        self.player.break_cooldown = 16;
                    }
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
            self.hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, self.player.inventory.selected);
        }

        if self.new_keys_down.contains(&KeyCode::KeyE) {
            if self.open_inventory.is_some() {
                self.open_inventory = None;
                self.open_inventory_model = None;
                lock_mouse(surface_ctx);
            } else {
                self.open_inventory = Some(OpenInventory::PlayerInventory);
                self.open_inventory_model = Some(create_inventory_model(surface_ctx, &OpenInventory::PlayerInventory));
                unlock_mouse(surface_ctx);
            }
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
        let chunk_pos = chunk_for_world_position(self.player.position.into());
        for cx in -1..=1 {
            for cy in -1..=1 {
                for cz in -1..=1 {
                    let chunk = self.chunk_manager.get_chunk_or_create([chunk_pos[0]+cx, chunk_pos[1]+cy, chunk_pos[2]+cz]);
                    let mut empty_list = vec![];
                    let item_entities = chunk.entities.get_mut(&EntityType::Item).unwrap_or(&mut empty_list);
                    let mut i = 0;
                    while i < item_entities.len() {
                        let entity = &item_entities[i];
                        if let TypedEntity::Item { stack } = &entity.entity_type && entity.position.distance2(self.player.position) < 4.0 {
                            if self.player.inventory.add(stack) {
                                item_entities.remove(i);
                                continue;
                            }
                        }
                        i += 1;
                    }
                }
            }
        }

        let mut count = 0;
        for chunk_pos in util::positions(100) {
            let relative_pos = [chunk_pos[0] + (self.camera.eye.x / CHUNK_SIZE as f32).floor() as i32, chunk_pos[1] + (self.camera.eye.y / CHUNK_SIZE as f32).floor() as i32, chunk_pos[2] + (self.camera.eye.z / CHUNK_SIZE as f32).floor() as i32];
            if !self.chunk_manager.chunk_loaded(relative_pos) {
                self.chunk_manager.get_chunk_or_create(relative_pos);
                self.chunk_manager.generate_blocks(relative_pos, self.player.position.into());
                count += 1;
                if count == 3 {
                    break;
                }
            }
        }
        self.chunk_manager.poll_channels(self.player.position.into());

        //render setup 
        let mut text_sections = vec![];
        self.particle_manager.run_step(surface_ctx);
        let time = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()-self.start_time) as f32 / 1000.0;
        self.screen_info_binding.set_data(surface_ctx.queue(), ScreenInfo::new(self.screen_size, time, self.camera.to_raw()));

        if let Some((target_position, _)) = self.player.raycast(self.camera.get_forward_vec(), 6.0, &self.chunk_manager) {
            self.cube_outline_model = Some(cube_outline_model(surface_ctx.device(), self.camera.build_view_projection_matrix(), Vector3::<i32>::from(target_position).cast().unwrap()));
        } else {
            self.cube_outline_model = None;
        }
        let hotbar_atlas_subsections = self.player.inventory.items[0..9].iter().map(|item| {
            self.item_atlas.subsection_for_position(item.atlas_coordinates)
        }).collect::<Vec<[f32; 4]>>().try_into().unwrap();
        self.hotbar_atlas_subsections_uniform.set_data(surface_ctx.queue(), DynamicOffsetUniform { values: hotbar_atlas_subsections, alignment: self.hotbar_atlas_subsections_uniform.value.alignment });
        let hotbar_item_texts = self.player.inventory.items.iter().flat_map(|item| {
            if item.stack.count > 1 {
                Some(OwnedText::new(format!("{}", item.stack.count))
                    .with_scale(surface_ctx.config().height as f32 * hotbar_item_height()/4.0)
                    .with_color([1.0; 4]))
            } else {
                None
            }
        }).collect::<Vec<_>>();
        if let Some(open_inventory) = &self.open_inventory {
            let values = match open_inventory {
                OpenInventory::PlayerInventory => {
                    self.player.inventory.items.iter().map(|it| self.item_atlas.subsection_for_position(it.atlas_coordinates)).collect()
                }
            };
            let value = DynamicOffsetUniformVec { values, alignment: surface_ctx.device().limits().min_uniform_buffer_offset_alignment as usize };
            if self.open_inventory_item_subsections_uniform.is_none() {
                self.open_inventory_item_subsections_uniform = Some(UniformBinding::new(surface_ctx.device(), "Open Inventory Item Subsections", value, None));
            } else {
                self.open_inventory_item_subsections_uniform.as_mut().unwrap().set_data(surface_ctx.queue(), value);
            }
            text_sections.extend_from_slice(&text_sections_for_inventory(surface_ctx, &|i| { 
                match open_inventory {
                    OpenInventory::PlayerInventory => &self.player.inventory.items[i].stack
                }
             }, open_inventory));
        }
        let screen_aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
        text_sections.extend_from_slice(&hotbar_item_texts.into_iter().enumerate().map(|(i, it)| {
            OwnedSection::default()
                .add_text(it)
                .with_bounds((surface_ctx.config().width as f32 * hotbar_item_height()/screen_aspect_ratio, surface_ctx.config().height as f32 * hotbar_item_height()))
                .with_screen_position(((((i as f32 - 3.5 - 1.0/8.0) * hotbar_item_height()/screen_aspect_ratio)/2.0 + 0.5) * surface_ctx.config().width as f32, (1.0 - hotbar_item_height()/32.0)*surface_ctx.config().height as f32))
                .with_layout(Layout::default().h_align(wgpu_text::glyph_brush::HorizontalAlign::Right).v_align(wgpu_text::glyph_brush::VerticalAlign::Bottom))

        }).collect::<Vec<_>>());

        self.health_ui_models = generate_health_ui_models(surface_ctx, self.player.health);

        //clean up
        self.text_brush.queue(surface_ctx.device(), surface_ctx.queue(), &text_sections).unwrap();
        self.new_keys_down = vec![];
    }

    fn render<'a: 'b, 'b>(&'a mut self, surface_ctx: &'b dyn SurfaceCtx, render_pass: &mut RenderPass<'b>) {
        self.chunk_shader.bind(render_pass);
        render_pass.set_bind_group(0, &self.screen_info_binding.binding, &[]);
        render_pass.set_bind_group(1, &self.atlas_uniform.binding, &[]);
        for chunk in self.chunk_manager.chunks_sorted(self.camera.eye.into()) {
            if chunk.visible() {
                //TODO: move camera stuff out of particle manager
                chunk.render(render_pass, &self.entity_render_manager, &self.item_atlas, &self.chunk_shader, &self.particle_manager.camera_view_binding, &self.particle_manager.camera_projection_binding, &self.atlas_uniform, surface_ctx);
            }
        }

        if let Some(cube_outline_model) = &self.cube_outline_model {
            self.cube_outline_shader.bind(render_pass);
            cube_outline_model.render(render_pass);
        }

        self.particle_manager.render(&self.screen_info_binding, &self.camera, render_pass, surface_ctx);
    }

    fn config(&self) -> Option<WindowConfig> {
        Some(WindowConfig { background_color: Color { r: 36.0/255.0, g: 105.0/255.0, b: 245.0/255.0, a: 1.0}, enable_post_processing: true })
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
                if !self.new_keys_down.contains(&code) {
                    self.new_keys_down.push(code);
                }
            } else {
                if let Some(i) = self.keys_down.iter().position(|x| x == &code) {
                    self.keys_down.remove(i);
                }
            }
        }
        if let Code(KeyCode::Escape) = input_event.physical_key {
            unlock_mouse(surface_ctx);
        }
    }
    
    fn mouse_motion(&mut self, _surface_ctx: &dyn SurfaceCtx, delta: (f64, f64)) {
        if self.open_inventory.is_none() {
            self.camera.ground += (delta.0 / 500.0) as f32;
            self.camera.sky -= (delta.1 / 500.0) as f32;
            self.camera.sky = self.camera.sky.clamp(std::f32::consts::PI*-0.499, std::f32::consts::PI*0.499);
        }
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
            self.ui_shader.bind(render_pass);
            render_pass.set_bind_group(0, &self.ui_texture_uniform.binding, &[]);
            self.hotbar_background_ui_models[i].render(render_pass);
            self.item_ui_shader.bind(render_pass);
            render_pass.set_bind_group(0, &self.item_atlas.texture.binding, &[]);
            render_pass.set_bind_group(1, &self.hotbar_atlas_subsections_uniform.binding, &[self.hotbar_atlas_subsections_uniform.dynamic_offset_for_index(i)]);
            ui_model.render(render_pass);
        }
        self.ui_shader.bind(render_pass);
        render_pass.set_bind_group(0, &self.ui_texture_uniform.binding, &[]);
        self.crosshair_model.render(render_pass);
        for model in &self.health_ui_models {
            model.render(render_pass);
        }
        if let Some(open_inventory_model) = &self.open_inventory_model && let Some(open_inventory_item_subsections_uniform) = &self.open_inventory_item_subsections_uniform {
            open_inventory_model.container.render(render_pass);
            self.item_ui_shader.bind(render_pass);
            render_pass.set_bind_group(0, &self.item_atlas.texture.binding, &[]);
            for (i, item) in open_inventory_model.items.iter().enumerate() {
                render_pass.set_bind_group(1, &open_inventory_item_subsections_uniform.binding, &[open_inventory_item_subsections_uniform.dynamic_offset_for_index(i)]);
                item.render(render_pass);
            }
        }

        self.text_brush.draw(render_pass);
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
                    lock_mouse(surface_context);
                }
            },
            WindowEvent::Focused(focused) => {
                if *focused {
                    lock_mouse(surface_context);
                } else {
                    unlock_mouse(surface_context);
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

fn lock_mouse(surface_ctx: &dyn SurfaceCtx) {
    let _ = surface_ctx.window().set_cursor_grab(winit::window::CursorGrabMode::Locked);
    surface_ctx.window().set_cursor_visible(false);
}

fn unlock_mouse(surface_ctx: &dyn SurfaceCtx) {
    let _ = surface_ctx.window().set_cursor_grab(winit::window::CursorGrabMode::None);
    surface_ctx.window().set_cursor_visible(true);
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