use bespoke_engine::{binding::{Descriptor, UniformBinding}, camera::OrthographicCamera, model::Render, resource_loader::ResourceConst, shader::{Shader, UniformShaderInit}, surface_context::SurfaceCtx, texture::{DepthTexture, Texture}};
use cgmath::{Matrix4, Vector2, vec2, vec3};
use itertools::Itertools;
use rustc_hash::FxHashMap;
use wgpu::RenderPassDepthStencilAttachment;

use crate::{BLOCK_ATLAS_PNG_DIRT_SECTION, BLOCK_ATLAS_PNG_GRASS_SECTION, BLOCK_ATLAS_PNG_HEIGHT, BLOCK_ATLAS_PNG_LEAVES_SECTION, BLOCK_ATLAS_PNG_PLANKS_SECTION, BLOCK_ATLAS_PNG_RAINBOW_SECTION, BLOCK_ATLAS_PNG_STONE_SECTION, BLOCK_ATLAS_PNG_WATER_SECTION, BLOCK_ATLAS_PNG_WIDTH, BLOCK_ATLAS_PNG_WOOD_SECTION, GENERATED_BLOCK_ATLAS_PNG, GENERATED_CRAFTING_RECIPES_JSON, RES_SHADERS_BLOCK_RENDERER_WGSL, block_models::block_model, blocks::{self, Block, BlockID, NOT_RENDERED_LAYER, SOLID_LAYER, TRANSPARENT_LAYER}, crafting::{CraftingRecipe, CraftingRecipeJson}, game::{ScreenInfo, Vertex}, inventory::ItemStack, items::{self, BlockItem, Item, ItemID, ItemProperties}};

pub struct ItemAtlasRegistry {
    pub texture: UniformBinding<Texture>,
    depth_texture: DepthTexture,
    item_positions: FxHashMap<ItemID, Vector2<u32>>,
    block_renderer_screen_info: UniformBinding<ScreenInfo>,
    block_renderer_transform_matrix: UniformBinding<[[f32; 4]; 4]>,
    item_render_size: u32,
}

impl ItemAtlasRegistry {
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let texture = UniformBinding::new(surface_ctx.device(), "Item Atlas", Texture::blank_texture(surface_ctx.device(), 64*4, 64*4, surface_ctx.config().format, 1), None);
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
        let block_renderer_transform_matrix = UniformBinding::new(surface_ctx.device(), "Block Renderer Transform Matrix", [[0.0; 4]; 4], None);
        
        Self {
            item_positions: FxHashMap::default(),
            item_render_size: 64,
            depth_texture: DepthTexture::create_depth_texture(surface_ctx.device(), texture.value.size.width, texture.value.size.height, "Item Atlas Depth Texture", 1),
            texture,
            block_renderer_screen_info,
            block_renderer_transform_matrix,
        }
    }

    pub fn items_per_row(&self) -> u32 {
        self.texture.value.size.width/self.item_render_size
    }

    pub fn num_rows(&self) -> u32 {
        self.texture.value.size.height/self.item_render_size
    }

    pub fn fractional_item_size(&self) -> Vector2<f32> {
        vec2(1.0/self.items_per_row() as f32, 1.0/self.num_rows() as f32)
    }

    pub fn get_item(&self, item: &ItemID) -> Vector2<u32> {
        if let Some(pos) = self.item_positions.get(item) {
            return *pos;
        } else {
            panic!("no atlas entry for item {item:?}");
        }
    }

    pub fn subsection_for_position(&self, position: Vector2<u32>) -> [f32; 4] {
        [position.x as f32/self.items_per_row() as f32, position.y as f32/self.num_rows() as f32, self.fractional_item_size().x, self.fractional_item_size().y]
    }

    pub fn render_block(&self, surface_ctx: &dyn SurfaceCtx, block: Block, block_atlas: &UniformBinding<Texture>, block_renderer_shader: &Shader) {
        if block.layer == NOT_RENDERED_LAYER {
            return;
        }
        let mut encoder = surface_ctx.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let block_model = block_model(surface_ctx, block);
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture.value.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
            });
            block_renderer_shader.bind(&mut render_pass);
            render_pass.set_bind_group(0, &self.block_renderer_screen_info.binding, &[]);
            render_pass.set_bind_group(1, &block_atlas.binding, &[]);
            render_pass.set_bind_group(2, &self.block_renderer_transform_matrix.binding, &[]);
            block_model.render(&mut render_pass);
        }
        surface_ctx.queue().submit([encoder.finish()]);
    }

    fn viewport_subsection_matrix(
        start_x: f32,
        start_y: f32,
        width: f32,
        height: f32,
    ) -> Matrix4<f32> {
        let scale_x = width;
        let scale_y = height;

        let center_u = start_x + width * 0.5;
        let center_v = start_y + height * 0.5;

        let translate_x = center_u * 2.0 - 1.0;
        let translate_y = 1.0 - center_v * 2.0;

        let m = Matrix4::new(
            scale_x, 0.0,     0.0, 0.0,
            0.0,     scale_y, 0.0, 0.0,
            0.0,     0.0,     1.0, 0.0,
            translate_x, translate_y, 0.0, 1.0,
        );
        m
    }
}

pub struct Registries {
    pub block_atlas_texture: UniformBinding<Texture>,
    pub item_atlas_registry: ItemAtlasRegistry,
    pub item_registry: ItemRegistry,
    pub block_registry: BlockRegistry,
    pub crafting_registry: CraftingRecipeRegistry,
}

impl Registries {
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let block_atlas_texture = UniformBinding::new(surface_ctx.device(), "Block Atlas", Texture::from_bytes(surface_ctx.device(), surface_ctx.queue(), &GENERATED_BLOCK_ATLAS_PNG.load(), "Atlas", None, None).unwrap(), None);
        let item_atlas_registry = ItemAtlasRegistry::new(surface_ctx);
        let mut item_registry = ItemRegistry::new();
        let mut block_registry = BlockRegistry::new();
        let mut crafting_registry = CraftingRecipeRegistry::new();

        block_registry.register_all();
        item_registry.register_all();
        for block in block_registry.blocks.values() {
            if let Some(item_id) = block.item {
                item_registry.register(Item { id: item_id, properties: ItemProperties::BlockItem(BlockItem { block: block.id }) });
            }
        }
        crafting_registry.register_all(&item_registry);

        let mut _self = Self {
            block_atlas_texture,
            item_atlas_registry,
            item_registry,
            block_registry,
            crafting_registry,
        };

        
        _self.insert_items_in_atlas(surface_ctx);

        _self
    }

    fn insert_items_in_atlas(&mut self, surface_ctx: &dyn SurfaceCtx) {
        let shader_atlas_x_blocks = ResourceConst { name: "ATLAS_X_BLOCKS".into(), rtype: "u32".into(), value: (BLOCK_ATLAS_PNG_WIDTH / BLOCK_ATLAS_PNG_DIRT_SECTION.width).to_string() };
        let shader_atlas_y_blocks  = ResourceConst { name: "ATLAS_Y_BLOCKS".into(), rtype: "u32".into(), value: (BLOCK_ATLAS_PNG_HEIGHT / BLOCK_ATLAS_PNG_DIRT_SECTION.height).to_string() };
        let block_renderer_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_BLOCK_RENDERER_WGSL, formats: vec![surface_ctx.config().format], uniforms: vec![&self.item_atlas_registry.block_renderer_screen_info, &self.block_atlas_texture, &self.item_atlas_registry.block_renderer_transform_matrix], shader_consts: vec![shader_atlas_x_blocks, shader_atlas_y_blocks], vertex_buffers: vec![Vertex::desc()], multisample_count: 1, ..Default::default() }, surface_ctx.device());
        
        for item in self.item_registry.items.values() {
            let item_atlas_registry = &mut self.item_atlas_registry;
            let next_position = vec2((item_atlas_registry.item_positions.len()) as u32 %item_atlas_registry.items_per_row(), (item_atlas_registry.item_positions.len()) as u32 /item_atlas_registry.items_per_row());
            let fractional_position = vec2(next_position.x as f32 / item_atlas_registry.items_per_row() as f32, next_position.y as f32 / item_atlas_registry.num_rows() as f32);
            item_atlas_registry.block_renderer_transform_matrix.set_data(surface_ctx.queue(), ItemAtlasRegistry::viewport_subsection_matrix(fractional_position.x, fractional_position.y, item_atlas_registry.fractional_item_size().x, item_atlas_registry.fractional_item_size().y).into());
            match &item.properties {
                ItemProperties::BlockItem(block_item) => {
                    item_atlas_registry.render_block(surface_ctx, self.block_registry.get_block(&block_item.block), &self.block_atlas_texture, &block_renderer_shader);
                },
                ItemProperties::Nothing => {},
            }
            item_atlas_registry.item_positions.insert(item.id, next_position);
        }
    }
}

pub struct CraftingRecipeRegistry {
    recipes: FxHashMap<Vec<String>, Vec<CraftingRecipe>>,
}

impl CraftingRecipeRegistry {
    pub fn new() -> Self {
        Self {
            recipes: FxHashMap::default(),
        }
    }

    pub fn register_all(&mut self, item_registry: &ItemRegistry) {
        let recipes_string = GENERATED_CRAFTING_RECIPES_JSON.load_string();
        let recipes_json: Vec<CraftingRecipeJson> = serde_json::from_str(&recipes_string).unwrap();
        for recipe_json in recipes_json {
            self.register(CraftingRecipe::from_json(recipe_json, item_registry));
        }
    }

    pub fn register(&mut self, recipe: CraftingRecipe) {
        let ingredients: Vec<String> = recipe.pattern.iter().flatten().map(|stack| stack.item.to_string()).unique().collect();
        if !self.recipes.contains_key(&ingredients) {
            self.recipes.insert(ingredients.clone(), vec![]);
        }
        self.recipes.get_mut(&ingredients).unwrap().push(recipe);
    }

    pub fn get_recipe_for_pattern(&self, pattern: Vec<Vec<ItemStack>>) -> Option<CraftingRecipe> {
        let ingredients: Vec<String> = pattern.iter().flatten().map(|stack| stack.item.to_string()).unique().collect();
        if let Some(possible_matches) = self.recipes.get(&ingredients) {
            for possible_match in possible_matches {
                if possible_match.pattern == pattern {
                    return Some(possible_match.clone());
                }
            }
        }
        return None;
    }
}

pub struct ItemRegistry {
    items: FxHashMap<String, Item>,
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self {
            items: FxHashMap::default(),
        }
    }

    pub fn get_item(&self, id: impl Into<String>) -> Item {
        *self.items.get(&id.into()).unwrap()
    }

    pub fn register_all(&mut self) {
        self.register(Item {
            id: items::NOTHING,
            properties: ItemProperties::Nothing,
        });
    }

    pub fn register(&mut self, item: Item) {
        self.items.insert(item.id.to_string(), item);
    }
}

pub struct BlockRegistry {
    blocks: FxHashMap<BlockID, Block>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: FxHashMap::default(),
        }
    }

    pub fn register_all(&mut self) {
        self.register(Block {
            id: blocks::AIR,
            solid: false,
            color: [0.0; 4],
            atlas_section: BLOCK_ATLAS_PNG_DIRT_SECTION,
            has_model: false,
            layer: NOT_RENDERED_LAYER,
            cull: true,
            item: None,
        });
        self.register(Block {
            id: blocks::GRASS,
            solid: true,
            color: [0.0, 1.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_GRASS_SECTION,
            has_model: false,
            layer: SOLID_LAYER,
            cull: true,
            item: Some("grass_block"),
        });
        self.register(Block {
            id: blocks::WATER,
            solid: false,
            color: [0.0, 0.0, 1.0, 0.5],
            atlas_section: BLOCK_ATLAS_PNG_WATER_SECTION,
            has_model: false,
            layer: TRANSPARENT_LAYER,
            cull: true,
            item: None,
        });
        self.register(Block {
            id: blocks::STONE,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_STONE_SECTION,
            has_model: false,
            layer: SOLID_LAYER,
            cull: true,
            item: Some("stone_block"),
        });
        self.register(Block {
            id: blocks::DIRT,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_DIRT_SECTION,
            has_model: false,
            layer: SOLID_LAYER,
            cull: true,
            item: Some("dirt_block"),
        });
        self.register(Block {
            id: blocks::ROCK,
            solid: false,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_STONE_SECTION,
            has_model: true,
            layer: SOLID_LAYER,
            cull: true,
            item: Some("rock_block"),
        });
        self.register(Block {
            id: blocks::LEAVES,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_LEAVES_SECTION,
            has_model: false,
            layer: SOLID_LAYER,
            cull: false,
            item: Some("leaves_block"),
        });
        self.register(Block {
            id: blocks::GOLD,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_RAINBOW_SECTION,
            has_model: false,
            layer: SOLID_LAYER,
            cull: true,
            item: Some("gold_block"),
        });
        self.register(Block {
            id: blocks::WOOD,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_WOOD_SECTION,
            has_model: false,
            layer: SOLID_LAYER,
            cull: true,
            item: Some("wood_block"),
        });
        self.register(Block {
            id: blocks::PLANKS,
            solid: true,
            color: [0.0, 0.0, 0.0, 1.0],
            atlas_section: BLOCK_ATLAS_PNG_PLANKS_SECTION,
            has_model: false,
            layer: SOLID_LAYER,
            cull: true,
            item: Some("planks_block"),
        });
    }

    fn register(&mut self, block: Block) {
        self.blocks.insert(block.id, block);
    }

    pub fn get_block(&self, id: &BlockID) -> Block {
        *self.blocks.get(id).unwrap()
    }
}