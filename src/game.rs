use std::{collections::HashMap, f32::consts::PI, sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}};

use bespoke_engine::{binding::{Binding, Descriptor, DynamicOffsetUniform, DynamicOffsetUniformVec, UniformBinding, WgslType, create_layout, simple_layout_entry}, camera::{Camera, CameraRaw}, culling::{AABB, vec3_mul_elements}, model::{Model, Render}, resource_loader::{ResourceConst, load_resource, load_resource_string}, shader::{PostProcessShaderInit, Shader, ShaderType, UniformShaderInit}, surface_context::SurfaceCtx, texture::{DepthTexture, Texture, TextureLayoutConfig}, window::{BasicVertex, MULTISAMPLE_COUNT, RenderStage, SurfaceConfig, WindowConfig, WindowHandler}};
use bytemuck::{NoUninit, Pod, Zeroable, bytes_of};
use glam::{IVec3, UVec2, Vec2, Vec3, ivec3, vec2, vec3};
use rustc_hash::FxHashMap;
use wgpu::{Color, CommandEncoder, Features, Limits, RenderPass, TextureFormat, wgt::CommandEncoderDescriptor};
use wgpu_text::{BrushBuilder, TextBrush, glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign, ab_glyph::FontVec}};
use winit::{dpi::PhysicalPosition, event::{KeyEvent, Modifiers, MouseButton, TouchPhase, WindowEvent}, keyboard::{KeyCode, PhysicalKey::Code}};
use crate::{BLOCK_ATLAS_PNG_DIRT_SECTION, BLOCK_ATLAS_PNG_HEIGHT, BLOCK_ATLAS_PNG_WIDTH, RES_SHADERS_BLUR_WGSL, RES_SHADERS_CHUNK_WGSL, RES_SHADERS_DEFERRED_COMBINE_WGSL, RES_SHADERS_ITEM_UI_WGSL, RES_SHADERS_POST_PROCESS_WGSL, RES_SHADERS_SSAO_WGSL, RES_SHADERS_UI_WGSL, RESOURCES, blocks::AIR, chunk::{CHUNK_SIZE, ChunkManager, NeededChunkUpdate}, const_block_models::Vertex, cube_outline::{cube_outline_model, cube_outline_shader}, entity::{Entity, EntityRenderManager, EntityType, TypedEntity}, game_serializer::{GameSerializer, WorldInfo}, inventory::{Inventory, InventoryInteraction, InventoryItemStack, ItemStack, cursor_stack_interaction}, items::{self, ItemProperties}, particles::{Particle, ParticleManager, ParticleType}, player::Player, registries::Registries, ssao::{SSAOKernelSamples, generate_random_texture, generate_ssao_kernel_samples}, ui::{InventoryLocation, InventoryLocationMut, InventoryModel, OpenInventory, UIVertex, create_inventory_model, generate_crosshair_ui_model, generate_health_ui_models, generate_hotbar_background_ui_models, generate_hotbar_item_ui_models, hotbar_item_height, inventory_location, inventory_physical_size, mouse_tile_coords, text_sections_for_inventory}, util::{self, chunk_for_block_position, chunk_for_world_position}};

const BLUR_STEPS: i32 = 11;

pub struct Game<'a> {
    pub deferred_color_output: UniformBinding<Texture>,
    pub deferred_normal_output: UniformBinding<Texture>,
    pub deferred_normal_resolve: UniformBinding<Texture>,
    pub deferred_worldspace_output: UniformBinding<Texture>,
    pub deferred_worldspace_resolve: UniformBinding<Texture>,
    pub deferred_ssao_output: UniformBinding<Texture>,
    pub intermediate_ssao_texture: UniformBinding<Texture>,
    pub deferred_depth_texture: UniformBinding<DepthTexture>,
    pub deferred_combine_shader: Shader<'a>,
    pub ssao_shader: Shader<'a>,
    pub ssao_blur_shader: Shader<'a>,

    pub ssao_kernel_samples: UniformBinding<SSAOKernelSamples>,
    pub random_texture: UniformBinding<Texture>,

    pub blur_radius: UniformBinding<[f32; 2]>,
    pub blur_steps: UniformBinding<i32>,
    pub blur_axis: UniformBinding<[f32; 2]>,
    pub blur_kernel: UniformBinding<BlurKernel>,
    
    pub screen_info_binding: UniformBinding<ScreenInfo>,

    pub screen_size: [f32; 2],
    pub mouse_coords: Vec2,
    pub start_time: u128,

    pub keys_down: Vec<KeyCode>,
    pub new_keys_down: Vec<KeyCode>,
    pub mouse_down: Vec<MouseButton>,
    pub new_mouse_down: Vec<MouseButton>,
    pub touch_positions: HashMap<u64, PhysicalPosition<f64>>,
    pub moving_bc_finger: Option<u64>,
    pub post_processing_shader: Shader<'a>,

    pub particle_manager: ParticleManager<'a>,

    pub world_info: WorldInfo,
    pub player: Player,

    pub chunk_manager: ChunkManager,
    pub chunk_shader: Shader<'a>,
    pub entity_render_manager: EntityRenderManager<'a>,

    pub registries: Arc<Registries>,

    pub cube_outline_shader: Shader<'a>,
    pub cube_outline_model: Option<Model>,
    pub hotbar_item_ui_models: Vec<Model>,
    pub hotbar_background_ui_models: Vec<Model>,
    pub health_ui_models: Vec<Model>,
    pub crosshair_model: Model,
    pub ui_shader: Shader<'a>,
    pub hotbar_atlas_subsections_uniform: UniformBinding<DynamicOffsetUniform<[f32; 4], 9>>,
    pub item_ui_shader: Shader<'a>,
    pub ui_texture_uniform: UniformBinding<Texture>,
    pub text_brush: TextBrush<FontVec>,
    pub text_sections: Vec<OwnedSection>,
    pub open_inventory: Option<OpenInventory>,
    pub weird_player_inventory_reference: OpenInventory,
    pub open_inventory_model: Option<InventoryModel>,
    pub open_inventory_item_subsections_uniform: Option<UniformBinding<DynamicOffsetUniformVec<[f32; 4], 0>>>,
    pub cursor_stack: InventoryItemStack,
    pub cursor_stack_model: Model,
    pub cursor_stack_subsection_uniform: UniformBinding<DynamicOffsetUniform<[f32; 4], 1>>,
}


impl <'a> Game<'a> {
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let deferred_data_format = TextureFormat::Rgba16Float;
        let ssao_format = TextureFormat::Rgba16Float;
        let deferred_normal_output = UniformBinding::new(surface_ctx.device(), "Deferred Normal Output", Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, deferred_data_format, MULTISAMPLE_COUNT.lock().unwrap().clone()), None);
        let deferred_normal_resolve = UniformBinding::new(surface_ctx.device(), "Deferred Normal Resolve", Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, deferred_data_format, 1), None);
        let deferred_color_output = UniformBinding::new(surface_ctx.device(), "Deferred Color Output", Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, surface_ctx.config().format, MULTISAMPLE_COUNT.lock().unwrap().clone()), None);
        let deferred_worldspace_output = UniformBinding::new(surface_ctx.device(), "Deferred Worldspace Output", Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, deferred_data_format, MULTISAMPLE_COUNT.lock().unwrap().clone()), None);
        let deferred_worldspace_resolve = UniformBinding::new(surface_ctx.device(), "Deferred Worldspace Resolve", Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, deferred_data_format, 1), None);
        let deferred_depth_texture = UniformBinding::new(surface_ctx.device(), "Deferred Depth Texture", DepthTexture::create_depth_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, "Deferred", MULTISAMPLE_COUNT.lock().unwrap().clone()), None);
        let intermediate_ssao_texture = UniformBinding::new(surface_ctx.device(), "Intermediate SSAO Texture", Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, ssao_format, 1), None);
        let deferred_ssao_output = UniformBinding::new(surface_ctx.device(), "Deferred SSAO Output", Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, ssao_format, 1), None);
        let deferred_formats = vec![surface_ctx.config().format, deferred_data_format, deferred_data_format];

        let ssao_kernel_samples = UniformBinding::new(surface_ctx.device(), "SSAO Kernel Samples", generate_ssao_kernel_samples(), None);
        let random_texture = UniformBinding::new(surface_ctx.device(), "Random Texture", generate_random_texture(surface_ctx, 64, 64, TextureFormat::Rgba32Float), None);

        let screen_size = [surface_ctx.size().0 as f32, surface_ctx.size().1 as f32];
        let camera = Camera {
            eye: vec3(-1.0, 0.0, 0.0),
            aspect: screen_size[0] / screen_size[1],
            fovy: 70.0,
            znear: 0.1,
            zfar: 10000.0,
            ground: 0.0,
            sky: 0.0,
        };
        let screen_info_binding = UniformBinding::new(surface_ctx.device(), "Screen Info", ScreenInfo::new(screen_size, 0.0, camera.to_raw()), None);
        let deferred_combine_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_DEFERRED_COMBINE_WGSL, formats: vec![surface_ctx.config().format], uniforms: vec![&deferred_color_output, &deferred_normal_output, &deferred_worldspace_output, surface_ctx.depth_texture(), &deferred_ssao_output, &screen_info_binding], vertex_buffers: vec![BasicVertex::desc()], depth_compare: wgpu::CompareFunction::Always,  ..Default::default() }, surface_ctx.device());
        let ssao_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_SSAO_WGSL, formats: vec![ssao_format], uniforms: vec![&deferred_normal_resolve, &deferred_worldspace_resolve, &screen_info_binding, &ssao_kernel_samples, &random_texture], vertex_buffers: vec![BasicVertex::desc()], enable_depth_texture: false, multisample_count: 1, ..Default::default() }, surface_ctx.device());
        
        let blur_steps = UniformBinding::new(surface_ctx.device(), "Blur Steps", BLUR_STEPS, None);
        let blur_radius = UniformBinding::new(surface_ctx.device(), "Blur Radius", [1.0 / surface_ctx.size().0 as f32, 1.0 / surface_ctx.size().1 as f32], None);
        let blur_axis = UniformBinding::new(surface_ctx.device(), "Blur Axis", [0.0; 2], None);
        let blur_kernel = UniformBinding::new(surface_ctx.device(), "Blur Kernel", generate_blur_kernel(), None);
        let ssao_blur_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_BLUR_WGSL, formats: vec![ssao_format], uniforms: vec![&intermediate_ssao_texture, &blur_radius, &blur_steps, &blur_axis, &blur_kernel], vertex_buffers: vec![BasicVertex::desc()], enable_depth_texture: false, multisample_count: 1, ..Default::default() }, surface_ctx.device());
        
        let mut registries = Arc::new(Registries::new(surface_ctx));
        let mut game_serializer = GameSerializer::new();

        let world_info = game_serializer.world_info.clone().unwrap_or_else(|| {
            WorldInfo { 
                seed: rand::random()
            }
        });
        let chunk_manager = ChunkManager::new(world_info.seed, registries.clone(), surface_ctx);

        //TODO: don't assume they are all the same width and height
        let shader_atlas_x_blocks = ResourceConst { name: "ATLAS_X_BLOCKS".into(), rtype: "u32".into(), value: (BLOCK_ATLAS_PNG_WIDTH / BLOCK_ATLAS_PNG_DIRT_SECTION.width).to_string() };
        let shader_atlas_y_blocks  = ResourceConst { name: "ATLAS_Y_BLOCKS".into(), rtype: "u32".into(), value: (BLOCK_ATLAS_PNG_HEIGHT / BLOCK_ATLAS_PNG_DIRT_SECTION.height).to_string() };

        let shader_atlas_x_ui = ResourceConst { name: "ATLAS_X_BLOCKS".into(), rtype: "u32".into(), value: 16.to_string() };
        let shader_atlas_y_ui  = ResourceConst { name: "ATLAS_Y_BLOCKS".into(), rtype: "u32".into(), value: 16.to_string() };

        let post_processing_shader = Shader::new(PostProcessShaderInit { resource: RES_SHADERS_POST_PROCESS_WGSL, formats: vec![surface_ctx.config().format], binding_layouts: vec![create_layout::<Texture>(TextureLayoutConfig::default(), surface_ctx.device())], shader_types: vec![Texture::shader_type(TextureLayoutConfig::default())], ..Default::default() }, surface_ctx.device());
        
        let chunk_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_CHUNK_WGSL, formats: deferred_formats.clone(), uniforms: vec![&screen_info_binding, &registries.block_atlas_texture], vertex_buffers: vec![Vertex::desc()], shader_consts: vec![shader_atlas_x_blocks.clone(), shader_atlas_y_blocks.clone()], line_mode: wgpu::PolygonMode::Fill, ..Default::default() }, surface_ctx.device());

        let cube_outline_shader = cube_outline_shader(surface_ctx.device(), deferred_formats.clone(), &screen_info_binding);
        let cube_outline_model = None;

        let hotbar_item_ui_models = generate_hotbar_item_ui_models(surface_ctx);
        let hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, 0);
        let health_ui_models = generate_health_ui_models(surface_ctx, 20.0);
        let ui_texture_uniform = UniformBinding::new(surface_ctx.device(), "UI Textures", Texture::from_bytes(surface_ctx.device(), surface_ctx.queue(), &load_resource("res/ui.png").unwrap(), "UI", None, None).unwrap(), None);
        let ui_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_UI_WGSL, formats: vec![surface_ctx.config().format], uniforms: vec![&ui_texture_uniform], vertex_buffers: vec![UIVertex::desc()], shader_consts: vec![shader_atlas_x_ui.clone(), shader_atlas_y_ui.clone()], enable_depth_texture: false, multisample_count: 1, ..Default::default() }, surface_ctx.device());
        let hotbar_atlas_subsections_uniform = UniformBinding::new(surface_ctx.device(), "Atlas Subsection", DynamicOffsetUniform { values: [[0.0; 4]; 9], alignment: surface_ctx.device().limits().min_uniform_buffer_offset_alignment as usize }, None);
        
        let item_ui_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_ITEM_UI_WGSL, formats: vec![surface_ctx.config().format], uniforms: vec![&registries.item_atlas_registry.texture, &hotbar_atlas_subsections_uniform], vertex_buffers: vec![UIVertex::desc()], shader_consts: vec![shader_atlas_x_ui.clone(), shader_atlas_y_ui.clone()], enable_depth_texture: false, multisample_count: 1, ..Default::default() }, surface_ctx.device());

        let crosshair_model = generate_crosshair_ui_model(surface_ctx);

        let particle_manager = ParticleManager::new(surface_ctx, &screen_info_binding, deferred_formats.clone());
        let entity_render_manager = EntityRenderManager::new(surface_ctx, deferred_formats.clone());
        
        let text_brush = BrushBuilder::using_font(FontVec::try_from_vec(load_resource("res/unifont.ttf").unwrap()).unwrap()).build(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, surface_ctx.config().format);
        let cursor_stack = InventoryItemStack::new(ItemStack::EMPTY, &mut registries);
        let cursor_stack_model = Model::new_empty::<u16>(AABB::zero(), surface_ctx.device());
        let cursor_stack_subsection_uniform = UniformBinding::new(surface_ctx.device(), "Cursor Stack Subsection", DynamicOffsetUniform { values: [[0.0; 4]], alignment: surface_ctx.device().limits().min_uniform_buffer_offset_alignment as usize }, None);

        let mut _self = Self {
            deferred_color_output,
            deferred_normal_output,
            deferred_normal_resolve,
            deferred_worldspace_output,
            deferred_worldspace_resolve,
            intermediate_ssao_texture,
            deferred_ssao_output,
            deferred_depth_texture,
            deferred_combine_shader,
            ssao_shader,
            ssao_blur_shader,
            blur_axis,
            blur_radius,
            blur_steps,
            blur_kernel,
            ssao_kernel_samples,
            random_texture,
            world_info,
            player: Player::new(vec3(0.0, 20.0, 0.0), camera, &mut registries),
            screen_size,
            mouse_coords: vec2(0.0, 0.0),
            screen_info_binding,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            keys_down: vec![],
            new_keys_down: vec![],
            mouse_down: vec![],
            new_mouse_down: vec![],
            touch_positions: HashMap::new(),
            moving_bc_finger: None,
            post_processing_shader,
            chunk_shader,
            chunk_manager,
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
            registries,
            hotbar_atlas_subsections_uniform,
            item_ui_shader,
            text_brush,
            text_sections: vec![],
            open_inventory: None,
            open_inventory_model: None,
            open_inventory_item_subsections_uniform: None,
            weird_player_inventory_reference: OpenInventory::PlayerInventory,
            cursor_stack,
            cursor_stack_model,
            cursor_stack_subsection_uniform,
        };

        game_serializer.load_world(&mut _self);

        if !_self.chunk_manager.get_chunk_or_create([0, -1, 0]).data.generated_blocks {
            _self.chunk_manager.generate_blocks([0, -1, 0], [0.0; 3]);
        }
        if !_self.chunk_manager.get_chunk_or_create([0; 3]).data.generated_blocks {
            _self.chunk_manager.generate_blocks([0; 3], [0.0; 3]);
        }
        if !_self.chunk_manager.get_chunk_or_create([0, 1, 0]).data.generated_blocks {
            _self.chunk_manager.generate_blocks([0, 1, 0], [0.0; 3]);
        }

        _self
    }
}

impl <'s> WindowHandler for Game<'s> {
    fn resize(&mut self, surface_ctx: &dyn SurfaceCtx, new_size: UVec2) {
        let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
        self.player.camera.aspect = aspect_ratio;
        self.screen_size = [new_size.x as f32, new_size.y as f32];
        self.hotbar_item_ui_models = generate_hotbar_item_ui_models(surface_ctx);
        self.crosshair_model = generate_crosshair_ui_model(surface_ctx);
        self.hotbar_background_ui_models = generate_hotbar_background_ui_models(surface_ctx, self.player.inventory.selected);
        self.text_brush.resize_view(surface_ctx.config().width as f32, surface_ctx.config().height as f32, surface_ctx.queue());
        self.deferred_normal_output.replace_data(surface_ctx.device(), Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, self.deferred_normal_output.value.format, MULTISAMPLE_COUNT.lock().unwrap().clone()));
        self.deferred_normal_resolve.replace_data(surface_ctx.device(), Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, self.deferred_normal_resolve.value.format, 1));
        self.deferred_color_output.replace_data(surface_ctx.device(), Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, self.deferred_color_output.value.format, MULTISAMPLE_COUNT.lock().unwrap().clone()));
        self.deferred_worldspace_output.replace_data(surface_ctx.device(), Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, self.deferred_worldspace_output.value.format, MULTISAMPLE_COUNT.lock().unwrap().clone()));
        self.deferred_worldspace_resolve.replace_data(surface_ctx.device(), Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, self.deferred_worldspace_resolve.value.format, 1));
        self.deferred_ssao_output.replace_data(surface_ctx.device(), Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, self.deferred_ssao_output.value.format, 1));
        self.deferred_depth_texture.replace_data(surface_ctx.device(), DepthTexture::create_depth_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, "Deferred", MULTISAMPLE_COUNT.lock().unwrap().clone()));
        self.blur_radius.set_data(surface_ctx.queue(), [1.0 / new_size.x as f32, 1.0 / new_size.y as f32]);
    }

    fn update(&mut self, surface_ctx: &dyn SurfaceCtx, delta: Duration) {
        self.update_user_input(surface_ctx, delta);
        self.update_player(surface_ctx, delta);
        self.update_all_chunks(surface_ctx, delta);
        self.update_render_setup(surface_ctx, delta);
        self.update_clean_up(surface_ctx, delta);
        self.update_render_deferred(surface_ctx, delta);
    }

    fn render<'a: 'b, 'b>(&'a mut self, surface_ctx: &'b dyn SurfaceCtx, render_pass: &mut RenderPass<'b>) {
        self.deferred_combine_shader.bind(render_pass);
        render_pass.set_bind_group(0, &self.deferred_color_output.binding, &[]);
        render_pass.set_bind_group(1, &self.deferred_normal_output.binding, &[]);
        render_pass.set_bind_group(2, &self.deferred_worldspace_output.binding, &[]);
        render_pass.set_bind_group(3, &self.deferred_depth_texture.binding, &[]);
        render_pass.set_bind_group(4, &self.deferred_ssao_output.binding, &[]);
        render_pass.set_bind_group(5, &self.screen_info_binding.binding, &[]);
        surface_ctx.screen_model().render(render_pass);
    }

    fn config(&self) -> WindowConfig {
        WindowConfig { background_color: Color { r: 36.0/255.0, g: 105.0/255.0, b: 245.0/255.0, a: 1.0}, enabled_render_stages: vec![RenderStage::Main, RenderStage::PostProcessing], ..Default::default() }
    }

    fn mouse_moved(&mut self, _surface_ctx: &dyn SurfaceCtx, mouse_pos: PhysicalPosition<f64>) {
        self.mouse_coords = vec2(mouse_pos.x as f32, mouse_pos.y as f32);
    }

    fn mouse_input(&mut self, _surface_context: &dyn SurfaceCtx, element_state: &winit::event::ElementState, mouse_button: &winit::event::MouseButton) {
        if element_state.is_pressed() {
            if !self.mouse_down.contains(mouse_button) {
                self.mouse_down.push(*mouse_button);
            }
            if !self.new_mouse_down.contains(mouse_button) {
                self.new_mouse_down.push(*mouse_button);
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
        if let Code(KeyCode::KeyR) = input_event.physical_key {
            self.chunk_shader.reload_source(surface_ctx.device());
            self.cube_outline_shader.reload_source(surface_ctx.device());
            self.deferred_combine_shader.reload_source(surface_ctx.device());
            self.ssao_shader.reload_source(surface_ctx.device());
            self.item_ui_shader.reload_source(surface_ctx.device());
            self.post_processing_shader.reload_source(surface_ctx.device());
            self.ui_shader.reload_source(surface_ctx.device());
        }
    }
    
    fn mouse_motion(&mut self, _surface_ctx: &dyn SurfaceCtx, delta: (f64, f64)) {
        if self.open_inventory.is_none() {
            self.player.camera.ground += (delta.0 / 500.0) as f32;
            self.player.camera.sky -= (delta.1 / 500.0) as f32;
            self.player.camera.sky = self.player.camera.sky.clamp(std::f32::consts::PI*-0.499, std::f32::consts::PI*0.499);
        }
        self.mouse_coords += vec2(delta.0 as f32, delta.1 as f32);
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
        // render_pass.set_bind_group(0, &self.deferred_normal_resolve.binding, &[]);
        surface_ctx.screen_model().render(render_pass);

        for (i, ui_model) in self.hotbar_item_ui_models.iter().enumerate() {
            self.ui_shader.bind(render_pass);
            render_pass.set_bind_group(0, &self.ui_texture_uniform.binding, &[]);
            self.hotbar_background_ui_models[i].render(render_pass);
            self.item_ui_shader.bind(render_pass);
            render_pass.set_bind_group(0, &self.registries.item_atlas_registry.texture.binding, &[]);
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
            render_pass.set_bind_group(0, &self.registries.item_atlas_registry.texture.binding, &[]);
            for (i, item) in open_inventory_model.items.iter().enumerate() {
                render_pass.set_bind_group(1, &open_inventory_item_subsections_uniform.binding, &[open_inventory_item_subsections_uniform.dynamic_offset_for_index(i)]);
                item.render(render_pass);
            }
        }
        if self.cursor_stack.stack.count > 0 {
            self.item_ui_shader.bind(render_pass);
            render_pass.set_bind_group(0, &self.registries.item_atlas_registry.texture.binding, &[]);
            render_pass.set_bind_group(1, &self.cursor_stack_subsection_uniform.binding, &[0]);
            self.cursor_stack_model.render(render_pass);
        }

        self.text_brush.draw(render_pass);
    }
    
    fn limits() -> wgpu::Limits {
        Limits {
            max_texture_dimension_2d: 8976,
            max_bind_groups: 7,
            ..Default::default()
        }
    }
    
    fn other_window_event(&mut self, surface_context: &dyn SurfaceCtx, event: &winit::event::WindowEvent) {
        match event {
            WindowEvent::CursorEntered { .. } => {
                if surface_context.window().has_focus() && self.open_inventory.is_none() {
                    lock_mouse(surface_context);
                }
            },
            WindowEvent::Focused(focused) => {
                if self.open_inventory.is_none() {
                    if *focused {
                        lock_mouse(surface_context);
                    } else {
                        unlock_mouse(surface_context);
                    }
                }
            }
            _ => {}
        }
    }

    fn before_shutdown(&mut self, _surface_context: &dyn SurfaceCtx) {
        let mut game_serializer = GameSerializer::new();
        game_serializer.save_world(self);
        println!("saved game state");
    }

    fn required_features() -> wgpu::Features {
        Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | Features::POLYGON_MODE_LINE | Features::VERTEX_WRITABLE_STORAGE
    }

    fn surface_config() -> SurfaceConfig {
        SurfaceConfig {
            multisample_count: 4,
            ..Default::default()
        }
    }

    fn custom_shader_type_source() -> String {
        load_resource_string("res/shaders/custom_shader_types.wgsl").into()
    }

    fn resources() -> Option<&'static phf::Map<&'static str, bespoke_engine::resource_loader::ResourceType>> {
        Some(&RESOURCES)
    }
}

impl <'s> Game<'s> {
    fn update_user_input(&mut self, surface_ctx: &dyn SurfaceCtx, delta: Duration) {
        // let max_speed = 4.317; //walking
        let max_speed = 5.612; //running
        let speed = max_speed * 12.08324875059 * delta.as_secs_f32(); //coefficient found experimentally on desmos because I forgot calculus
        let mut movement = vec3(0.0, 0.0, 0.0);
        if self.keys_down.contains(&KeyCode::KeyW) || self.moving_bc_finger.is_some() {
            movement += self.player.camera.get_walking_vec();
        }
        if self.moving_bc_finger.is_some() {
            movement += self.player.camera.get_walking_vec();
        }
        if self.keys_down.contains(&KeyCode::KeyS) {
            movement -= self.player.camera.get_walking_vec();
        }
        if self.keys_down.contains(&KeyCode::KeyA) {
            movement -= self.player.camera.get_right_vec();
        }
        if self.keys_down.contains(&KeyCode::KeyD) {
            movement += self.player.camera.get_right_vec();
        }
        if self.keys_down.contains(&KeyCode::Space) {
            if self.player.time_since_ground < Duration::from_secs_f32(0.1) {
                self.player.velocity += Vec3::Y * 8.94427191;
                self.player.velocity += self.player.camera.get_walking_vec() * 80.0 * delta.as_secs_f32();
                self.player.time_since_ground = Duration::from_secs_f32(1.0);
            }
        }
        if self.player.movement_mode == 1 {
            if self.keys_down.contains(&KeyCode::ShiftLeft) {
                self.player.velocity -= Vec3::Y * speed;
            }
            if self.keys_down.contains(&KeyCode::Space) {
                self.player.velocity += Vec3::Y * speed;
            }
        }
        if movement.length_squared() > 0.0 {
            self.player.velocity += movement.normalize() * speed;
        }
        if self.keys_down.contains(&KeyCode::KeyC) {
            self.player.movement_mode = 1;
        }
        if self.keys_down.contains(&KeyCode::KeyV) {
            self.player.movement_mode = 0;
        }
        if self.open_inventory.is_none() {
            if self.mouse_down.contains(&MouseButton::Right) {
                if self.player.break_cooldown.is_zero() || self.new_mouse_down.contains(&MouseButton::Right) {
                    if let Some((coordinate, face)) = self.player.raycast(self.player.camera.get_forward_vec(), 6.0, &self.chunk_manager, &self.registries) {
                        if let ItemProperties::BlockItem(block_item) = self.registries.item_registry.get_item(self.player.inventory.selected_item().stack.item).properties && block_item.block != AIR {
                            let coordinate = (IVec3::from(coordinate)+face.direction()).into();
                            let before = self.chunk_manager.get_block(coordinate);
                            self.chunk_manager.set_block(coordinate, block_item.block, true);
                            if self.player.colliding_world(&self.chunk_manager, &self.registries) {
                                self.chunk_manager.set_block(coordinate, before, true);
                            } else {
                                self.player.inventory.selected_item_mut().stack.count -= 1;
                                if self.player.inventory.selected_item_mut().stack.count <= 0 {
                                    *self.player.inventory.selected_item_mut() = InventoryItemStack::new(ItemStack::EMPTY, &self.registries);
                                }
                            }
                            self.player.break_cooldown = Duration::from_secs_f32(0.2);
                        }
                    }
                }
            }
            if self.mouse_down.contains(&MouseButton::Left) {
                if self.player.break_cooldown.is_zero() || self.new_mouse_down.contains(&MouseButton::Left) {
                    if let Some((coordinate, _)) = self.player.raycast(self.player.camera.get_forward_vec(), 6.0, &self.chunk_manager, &self.registries) {
                        if self.player.break_position.is_none_or(|it| it == coordinate.into()) {
                            let before = self.chunk_manager.get_block(coordinate);
                            let before_block = self.registries.block_registry.get_block(&before);
                            if self.player.break_progress > before_block.break_duration {
                                self.chunk_manager.set_block(coordinate, AIR, true);
                                for _ in 0..10 {
                                    self.particle_manager.add_particle(Particle {
                                        particle_type: ParticleType::BlockBreak,
                                        position: (IVec3::from(coordinate).as_vec3()+vec3(rand::random_range(0.0..1.0), rand::random_range(0.0..1.0), rand::random_range(0.0..1.0))).extend(1.0).into(),
                                        velocity: (vec3(rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0))*2.0).into(),
                                        color: before_block.color,
                                        lifetime: Duration::from_secs_f32(20.0),
                                    });
                                }
                                if before_block.drops != items::NOTHING {
                                    self.chunk_manager.get_chunk_or_create(chunk_for_block_position(coordinate)).add_entity(Entity {
                                        entity_type: TypedEntity::Item { stack: InventoryItemStack::new(ItemStack::new(before_block.drops, 1), &self.registries) },
                                        position: IVec3::from(coordinate).as_vec3()+vec3(0.5, 0.5, 0.5),
                                        velocity: (vec3(rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0))*5.0).into(),
                                        time_alive: Duration::ZERO,
                                    });
                                }
                                self.player.break_cooldown = Duration::from_secs_f32(0.0);
                            } else {
                                self.player.add_break_progress(delta, before_block, &self.registries);
                                if rand::random_range(0.0..1.0) < 0.1*self.player.block_break_modifier(before_block, &self.registries) {
                                    self.particle_manager.add_particle(Particle {
                                        particle_type: ParticleType::BlockBreak,
                                        position: (IVec3::from(coordinate).as_vec3()+vec3(rand::random_range(0.0..1.0), rand::random_range(0.0..1.0), rand::random_range(0.0..1.0))).extend(1.0).into(),
                                        velocity: (vec3(rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0), rand::random_range(-1.0..1.0))*4.0).into(),
                                        color: before_block.color,
                                        lifetime: Duration::from_secs_f32(20.0),
                                    });
                                }
                            }
                        } else {
                            self.player.break_progress = Duration::ZERO;
                        }
                        self.player.break_position = Some(coordinate.into());
                    }
                }
            } else {
                self.player.break_position = None;
                self.player.break_progress = Duration::ZERO;
            }
        }
        if delta <= self.player.break_cooldown {
            self.player.break_cooldown = self.player.break_cooldown-delta;
        } else {
            self.player.break_cooldown = Duration::ZERO;
        }

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
                let drop_stacks = match self.open_inventory.as_ref().unwrap() {
                    OpenInventory::PlayerCrafting(crafting_inventory) => {
                        &crafting_inventory.items[0..9]
                    },
                    OpenInventory::PlayerInventory => &[]
                };
                for stack in drop_stacks {
                    self.chunk_manager.get_chunk_or_create(chunk_for_world_position(self.player.position.into())).add_entity(Entity {
                        entity_type: TypedEntity::Item { stack: stack.clone() },
                        position: self.player.position,
                        velocity: vec3_mul_elements(self.player.camera.get_forward_vec(), vec3(20.0, 5.0, 20.0)),
                        time_alive: Duration::ZERO,
                    });
                }
                self.open_inventory = None;
                self.open_inventory_model = None;
                lock_mouse(surface_ctx);
            } else {
                self.open_inventory = Some(OpenInventory::PlayerCrafting(Inventory::empty_size(10, &self.registries)));
                self.open_inventory_model = Some(create_inventory_model(surface_ctx, self.open_inventory.as_ref().unwrap()));
                unlock_mouse(surface_ctx);
            }
        }
        let mouse_interaction = if self.new_mouse_down.contains(&MouseButton::Left) {
            Some(InventoryInteraction::TakeStack)
        } else if self.new_mouse_down.contains(&MouseButton::Right) {
            Some(InventoryInteraction::TakeOne)
        } else {
            None
        };
        if let Some(mouse_interaction) = mouse_interaction {
            if let Some(open_inventory) = &mut self.open_inventory {
                if let Some(location) = mouse_tile_coords(vec2(self.mouse_coords.x/surface_ctx.config().width as f32, self.mouse_coords.y/surface_ctx.config().height as f32), surface_ctx, open_inventory) {
                    let mut location_mut = if let OpenInventory::PlayerInventory = location.inventory {
                        InventoryLocationMut { inventory: &mut self.weird_player_inventory_reference, x: location.x, y: location.y }
                    } else {
                        InventoryLocationMut { x: location.x, y: location.y, inventory: open_inventory }
                    };
                    Self::interact_inventory_slot(&mut location_mut, &mut self.player.inventory, &mut self.cursor_stack, mouse_interaction, &self.registries);
                }
            }
        }
    }

    fn update_player(&mut self, _surface_ctx: &dyn SurfaceCtx, delta: Duration) {
        if self.player.movement_mode == 0 {
            self.player.velocity.y -= 32.0 * delta.as_secs_f32();
        }
        let friction_coefficient = 0.00001f64;
        self.player.velocity.x *= friction_coefficient.powf(delta.as_secs_f64()) as f32;
        self.player.velocity.z *= friction_coefficient.powf(delta.as_secs_f64()) as f32;
        if self.player.movement_mode == 1 {
            self.player.velocity.y *= friction_coefficient.powf(delta.as_secs_f64()) as f32;
        }
        self.player.time_since_ground += delta;
        self.player.move_player(self.player.velocity * delta.as_secs_f32(), &self.chunk_manager, &self.registries);
        self.player.camera.eye = self.player.position;

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
                        if let TypedEntity::Item { stack } = &entity.entity_type && entity.time_alive > Duration::from_secs_f32(0.5) && entity.position.distance_squared(self.player.position) < 4.0 {
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
    }

    fn update_all_chunks(&mut self, surface_ctx: &dyn SurfaceCtx, delta: Duration) {
        let mut move_entities = vec![];
        let mut needed_chunk_updates = FxHashMap::default();
        for pos in self.chunk_manager.chunk_positions_owned() {
            for entity_type in self.chunk_manager.get_chunk_or_create(pos).entities.keys().cloned().collect::<Vec<EntityType>>() {
                let mut i = 0;
                let mut len = self.chunk_manager.get_chunk_or_create(pos).entities.get(&entity_type).unwrap().len();
                let mut temp_entity = Entity { position: vec3(0.0, 0.0, 0.0), velocity: vec3(0.0, 0.0, 0.0), entity_type: TypedEntity::Marker, time_alive: Duration::ZERO };
                while i < len {
                    std::mem::swap(&mut self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap()[i], &mut temp_entity);
                    temp_entity.update(&self.chunk_manager, delta, &self.registries);
                    std::mem::swap(&mut self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap()[i], &mut temp_entity);
                    if chunk_for_world_position(self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap()[i].position.into()) != pos {
                        move_entities.push(self.chunk_manager.get_chunk_or_create(pos).entities.get_mut(&entity_type).unwrap().remove(i));
                    } else {
                        i += 1;
                    }
                    len = self.chunk_manager.get_chunk_or_create(pos).entities.get(&entity_type).unwrap().len();
                }
            }
            let chunk = self.chunk_manager.get_chunk_or_create(pos);
            let mut this_chunk_updates = vec![];
            std::mem::swap(&mut this_chunk_updates, &mut chunk.needed_chunk_updates);
            if chunk.model.is_some() && !chunk.creating_model && ChunkManager::lod_for_distance2(pos, self.player.position.into()) != chunk.lod {
                this_chunk_updates.push(NeededChunkUpdate { relative_chunk_pos: ivec3(0, 0, 0), synchronous: false });
            }
            for needed_update in this_chunk_updates {
                let actual_chunk_pos = needed_update.relative_chunk_pos+IVec3::from(pos);
                if let Some(synchronous) = needed_chunk_updates.get(&actual_chunk_pos) {
                    if !synchronous && needed_update.synchronous {
                        needed_chunk_updates.insert(actual_chunk_pos, true);
                    }
                } else {
                    needed_chunk_updates.insert(actual_chunk_pos, needed_update.synchronous);
                }
           }
        }
        while let Some(entity) = move_entities.pop() {
            self.chunk_manager.get_chunk_or_create(chunk_for_world_position(entity.position.into())).add_entity(entity);
        }
        for (pos, synchronous) in needed_chunk_updates {
            if synchronous {
                self.chunk_manager.generate_model_now(pos.into(), self.player.position.into(), &self.registries, surface_ctx);
            } else {
                self.chunk_manager.generate_model(pos.into(), self.player.position.into());
            }
        }

        for chunk_pos in util::positions(50) {
            if self.chunk_manager.pending_requests > 5 {
                break;
            }
            let relative_pos = [chunk_pos[0] + (self.player.camera.eye.x / CHUNK_SIZE as f32).floor() as i32, chunk_pos[1] + (self.player.camera.eye.y / CHUNK_SIZE as f32).floor() as i32, chunk_pos[2] + (self.player.camera.eye.z / CHUNK_SIZE as f32).floor() as i32];
            if !self.chunk_manager.chunk_loaded(relative_pos) {
                self.chunk_manager.get_chunk_or_create(relative_pos);
                self.chunk_manager.generate_blocks(relative_pos, self.player.position.into());
            } else {
                let chunk = self.chunk_manager.get_chunk_or_create(relative_pos);
                if chunk.model.is_none() && !chunk.creating_model {
                    self.chunk_manager.generate_model(relative_pos, self.player.position.into());
                }
            }
        }
        self.chunk_manager.poll_channels(self.player.position.into());
    }

    fn update_render_setup(&mut self, surface_ctx: &dyn SurfaceCtx, delta: Duration) {
        self.particle_manager.run_step(delta, surface_ctx);
        let time = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()-self.start_time) as f32 / 1000.0;
        self.screen_info_binding.set_data(surface_ctx.queue(), ScreenInfo::new(self.screen_size, time, self.player.camera.to_raw()));

        if let Some((target_position, _)) = self.player.raycast(self.player.camera.get_forward_vec(), 6.0, &self.chunk_manager, &self.registries) {
            self.cube_outline_model = Some(cube_outline_model(surface_ctx.device(), self.player.camera.build_view_projection_matrix(), IVec3::from(target_position).as_vec3()));
        } else {
            self.cube_outline_model = None;
        }
        let hotbar_atlas_subsections = self.player.inventory.items[0..9].iter().map(|item| {
            self.registries.item_atlas_registry.fractional_atlas_section(&item.atlas_section)
        }).collect::<Vec<[f32; 4]>>().try_into().unwrap();
        self.hotbar_atlas_subsections_uniform.set_data(surface_ctx.queue(), DynamicOffsetUniform { values: hotbar_atlas_subsections, alignment: self.hotbar_atlas_subsections_uniform.value.alignment });
        if let Some(open_inventory) = &mut self.open_inventory {
            let mut values = vec![];
            let (num_physical_rows, num_physical_cols) = inventory_physical_size(open_inventory);
            for physical_y in 0..num_physical_rows {
                for physical_x in 0..num_physical_cols {
                    if let Some(location) = inventory_location(open_inventory, physical_x, physical_y) {
                        if let Some(stack) = Self::stack_at_location(&location, &self.player.inventory) {
                            values.push(self.registries.item_atlas_registry.fractional_atlas_section(&stack.atlas_section));
                        } else {
                            println!("got location {location:?} but no stack is associated");
                        }
                    }
                }
            }
            let value = DynamicOffsetUniformVec { values, alignment: surface_ctx.device().limits().min_uniform_buffer_offset_alignment as usize };
            if self.open_inventory_item_subsections_uniform.is_none() {
                self.open_inventory_item_subsections_uniform = Some(UniformBinding::new(surface_ctx.device(), "Open Inventory Item Subsections", value, None));
            } else {
                self.open_inventory_item_subsections_uniform.as_mut().unwrap().set_data(surface_ctx.queue(), value);
            }
            self.text_sections.extend_from_slice(&text_sections_for_inventory(surface_ctx, &|location| { 
                Self::stack_at_location(&location, &self.player.inventory).map(|it| it.stack.count)
             }, open_inventory));
        }

        let screen_aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
        let hotbar_item_text_sections = self.player.inventory.items[0..9].iter().enumerate().flat_map(|(i, item)| {
            if item.stack.count > 1 {
                Some(OwnedSection::default()
                    .add_text(OwnedText::new(format!("{}", item.stack.count))
                    .with_scale(surface_ctx.config().height as f32 * hotbar_item_height()/4.0)
                    .with_color([1.0; 4]))
                    .with_bounds((surface_ctx.config().width as f32 * hotbar_item_height()/screen_aspect_ratio, surface_ctx.config().height as f32 * hotbar_item_height()))
                    .with_screen_position(((((i as f32 - 3.5 - 1.0/8.0) * hotbar_item_height()/screen_aspect_ratio)/2.0 + 0.5) * surface_ctx.config().width as f32, (1.0 - hotbar_item_height()/32.0)*surface_ctx.config().height as f32))
                    .with_layout(Layout::default().h_align(wgpu_text::glyph_brush::HorizontalAlign::Right).v_align(wgpu_text::glyph_brush::VerticalAlign::Bottom)))
            } else {
                None
            }
        }).collect::<Vec<_>>();
        self.text_sections.extend_from_slice(&hotbar_item_text_sections);

        self.health_ui_models = generate_health_ui_models(surface_ctx, self.player.health);

        let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
        let cursor_stack_height = hotbar_item_height();
        let cursor_stack_width = cursor_stack_height / aspect_ratio;
        let cursor_screen_position = vec2((self.mouse_coords.x/surface_ctx.config().width as f32 - 0.5)*2.0, (-self.mouse_coords.y/surface_ctx.config().height as f32 + 0.5)*2.0);
        self.cursor_stack_model = Model::new(vec![
            UIVertex { position: [cursor_screen_position.x, cursor_screen_position.y-cursor_stack_height, 0.0], tex_coords: [0.0, 1.0], repeat_count: [1.0, 1.0] },
            UIVertex { position: [cursor_screen_position.x, cursor_screen_position.y, 0.0], tex_coords: [0.0, 0.0], repeat_count: [1.0, 1.0] },
            UIVertex { position: [cursor_screen_position.x+cursor_stack_width, cursor_screen_position.y-cursor_stack_height, 0.0], tex_coords: [1.0, 1.0], repeat_count: [1.0, 1.0] },
            UIVertex { position: [cursor_screen_position.x+cursor_stack_width, cursor_screen_position.y, 0.0], tex_coords: [1.0, 0.0], repeat_count: [1.0, 1.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device());
        self.cursor_stack_subsection_uniform.set_data(surface_ctx.queue(), DynamicOffsetUniform { values: [self.registries.item_atlas_registry.fractional_atlas_section(&self.cursor_stack.atlas_section)], alignment: self.cursor_stack_subsection_uniform.value.alignment });
        if self.cursor_stack.stack.count > 1 {
            self.text_sections.push(OwnedSection::default()
                .add_text(
                    OwnedText::new(format!("{}", self.cursor_stack.stack.count))
                    .with_scale(surface_ctx.config().height as f32 * cursor_stack_height/4.0)
                    .with_color([0.0, 0.0, 0.0, 1.0]))
                .with_bounds((cursor_stack_width * surface_ctx.config().width as f32, cursor_stack_height * surface_ctx.config().height as f32))
                .with_layout(Layout::default().h_align(HorizontalAlign::Right).v_align(VerticalAlign::Bottom))
                .with_screen_position((self.mouse_coords.x+cursor_stack_width/2.0*surface_ctx.config().width as f32, self.mouse_coords.y+cursor_stack_height/2.0*surface_ctx.config().height as f32))
            );
        }
    }

    fn update_clean_up(&mut self, surface_ctx: &dyn SurfaceCtx, _delta: Duration) {
        self.text_brush.queue(surface_ctx.device(), surface_ctx.queue(), &self.text_sections).unwrap();
        self.text_sections = vec![];
        self.new_keys_down = vec![];
        self.new_mouse_down = vec![];
    }

    fn update_render_deferred(&mut self, surface_ctx: &dyn SurfaceCtx, _delta: Duration) {
        let mut encoder = surface_ctx.device().create_command_encoder(&CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Deferred Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        resolve_target: None,
                        view: &self.deferred_color_output.value.view,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        resolve_target: Some(&self.deferred_normal_resolve.value.view),
                        view: &self.deferred_normal_output.value.view,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        resolve_target: Some(&self.deferred_worldspace_resolve.value.view),
                        view: &self.deferred_worldspace_output.value.view,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                ],
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.deferred_depth_texture.value.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
            });
            self.render_deferred(surface_ctx, &mut render_pass);
        }
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        resolve_target: None,
                        view: &self.deferred_ssao_output.value.view,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::RED),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                ],
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                depth_stencil_attachment: None
            });
            self.ssao_shader.bind(&mut render_pass);
            render_pass.set_bind_group(0, &self.deferred_normal_resolve.binding, &[]);
            render_pass.set_bind_group(1, &self.deferred_worldspace_resolve.binding, &[]);
            render_pass.set_bind_group(2, &self.screen_info_binding.binding, &[]);
            render_pass.set_bind_group(3, &self.ssao_kernel_samples.binding, &[]);
            render_pass.set_bind_group(4, &self.random_texture.binding, &[]);
            surface_ctx.screen_model().render(&mut render_pass);
        }
        for _ in 0..3 {
            self.blur_axis.set_data(surface_ctx.queue(), [1.0, 0.0]);
            self.blur_ssao(surface_ctx, &mut encoder);
            self.blur_axis.set_data(surface_ctx.queue(), [0.0, 1.0]);
            self.blur_ssao(surface_ctx, &mut encoder);
        }
        surface_ctx.queue().submit([encoder.finish()]);
    }

    fn render_deferred<'a: 'b, 'b>(&'a mut self, surface_ctx: &'b dyn SurfaceCtx, render_pass: &mut RenderPass<'b>) {
        self.chunk_shader.bind(render_pass);
        render_pass.set_bind_group(0, &self.screen_info_binding.binding, &[]);
        render_pass.set_bind_group(1, &self.registries.block_atlas_texture.binding, &[]);
        for chunk in self.chunk_manager.chunks_sorted(self.player.camera.eye.into()) {
            if chunk.visible() {
                //TODO: move camera stuff out of particle manager
                chunk.render(render_pass, &self.entity_render_manager, &self.registries, &self.chunk_shader, surface_ctx);
            }
        }

        if let Some(cube_outline_model) = &self.cube_outline_model {
            self.cube_outline_shader.bind(render_pass);
            cube_outline_model.render(render_pass);
        }

        self.particle_manager.render(&self.screen_info_binding, render_pass);
    }

    fn blur_ssao<'a: 'b, 'b>(&'a mut self, surface_ctx: &'b dyn SurfaceCtx, encoder: &mut CommandEncoder) {
        std::mem::swap(&mut self.deferred_ssao_output, &mut self.intermediate_ssao_texture);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SSAO Blur Render Pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    resolve_target: None,
                    view: &self.deferred_ssao_output.value.view,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })
            ],
            multiview_mask: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            depth_stencil_attachment: None
        });
        self.ssao_blur_shader.bind(&mut render_pass);
        render_pass.set_bind_group(0, &self.intermediate_ssao_texture.binding, &[]);
        render_pass.set_bind_group(1, &self.blur_radius.binding, &[]);
        render_pass.set_bind_group(2, &self.blur_steps.binding, &[]);
        render_pass.set_bind_group(3, &self.blur_axis.binding, &[]);
        render_pass.set_bind_group(4, &self.blur_kernel.binding, &[]);
        surface_ctx.screen_model().render(&mut render_pass);
    }

    fn stack_index_at_location<'a>(location: &'a InventoryLocation<'a>) -> Option<usize> {
        match location.inventory {
            OpenInventory::PlayerCrafting(_) =>  {
                if (0..3).contains(&location.y) && (0..3).contains(&location.x) {
                    let i = location.y * 3 + location.x;
                    Some(i as usize)
                } else if location.x == 4 && location.y == 1 {
                    Some(9)
                } else {
                    None
                }
            },
            OpenInventory::PlayerInventory => {
                let i = location.y * 9 + location.x;
                Some(i as usize)
            }
        }
    }

    fn stack_at_location<'a>(location: &'a InventoryLocation<'a>, player_inventory: &'a Inventory) -> Option<&'a InventoryItemStack> {
        if let Some(index) = Self::stack_index_at_location(location) {
            if location.inventory == &OpenInventory::PlayerInventory {
                return Some(&player_inventory.items[index])
            } else {
                return Some(Self::stack_at_index(index, location.inventory, player_inventory))
            }
        } else {
            return None
        }
    }

    fn stack_at_index<'a>(index: usize, inventory: &'a OpenInventory, player_inventory: &'a Inventory) -> &'a InventoryItemStack {
        match inventory {
            OpenInventory::PlayerInventory => &player_inventory.items[index],
            OpenInventory::PlayerCrafting(inventory) => &inventory.items[index],
        }
    }

    fn stack_at_index_mut<'a>(index: usize, inventory: &'a mut OpenInventory, player_inventory: &'a mut Inventory) -> &'a mut InventoryItemStack {
        match inventory {
            OpenInventory::PlayerInventory => &mut player_inventory.items[index],
            OpenInventory::PlayerCrafting(inventory) => &mut inventory.items[index],
        }
    }

    fn interact_inventory_slot<'a>(location: &'a mut InventoryLocationMut<'a>, player_inventory: &'a mut Inventory, cursor_stack: &mut InventoryItemStack, interaction: InventoryInteraction, registries: &Registries) {
        match location.inventory {
            OpenInventory::PlayerInventory => {
                if let Some(i) = Self::stack_index_at_location(&location.as_ref()) {
                    cursor_stack_interaction(cursor_stack, &mut player_inventory.items[i], interaction, registries);
                }
            },
            OpenInventory::PlayerCrafting(crafting_inventory) => {
                if location.x == 4 && location.y == 1 {
                    crafting_inventory.take_crafting_result(cursor_stack, registries);
                    crafting_inventory.calculate_crafting_result(registries);
                }
                if (0..3).contains(&location.x) && (0..3).contains(&location.y) {
                    if let Some(i) = Self::stack_index_at_location(&location.as_ref()) {
                        let stack = Self::stack_at_index_mut(i, location.inventory, player_inventory);
                        cursor_stack_interaction(cursor_stack, stack, interaction, registries);
                        if let Some(inventory) = location.inner_inventory() {
                            inventory.calculate_crafting_result(registries);
                        }
                    }
                }
            }
        }
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
    type LayoutConfig = ();
    fn layout_config(&self) -> Self::LayoutConfig {
        ()
    }
    fn create_resources<'a>(&'_ self) -> Vec<bespoke_engine::binding::Resource<'_>> {
        vec![bespoke_engine::binding::Resource::Simple(bytes_of(self).to_vec())]
    }

    fn layout(_config: (), _ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![simple_layout_entry(0)]
    }

    fn shader_type(_config: ()) -> bespoke_engine::shader::ShaderType {
        ShaderType {
            var_types: vec!["<uniform>".into()],
            wgsl_types: vec!["ScreenInfo".into()]
        }    
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct BlurKernel {
    value: [[f32; 4]; BLUR_STEPS as usize],
}

impl WgslType for BlurKernel {
    fn wgsl_name() -> String {
        "BlurKernel".into()
    }
}

fn generate_blur_kernel() -> BlurKernel {
    let mut kernel = [[0.0; 4]; BLUR_STEPS as usize];
    let stdev = (BLUR_STEPS as f32 - 1.0)/(2.0 * 1.96);
    let fract = 1.0 / (2.0 * PI * stdev.powi(2)).sqrt();
    for i in 0..kernel.len() {
        let x = i as f32 - (BLUR_STEPS-1) as f32 / 2.0;
        let g = fract*std::f32::consts::E.powf(-x.powi(2)/(2.0*stdev.powi(2)));
        kernel[i] = [g; 4];
    }
    BlurKernel { value: kernel }
}