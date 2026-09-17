use bespoke_engine::{binding::{Descriptor, UniformBinding}, camera::OrthographicCamera, model::Render, resource_compiler::AtlasSection, resource_loader::ResourceConst, shader::{Shader, UniformShaderInit}, surface_context::SurfaceCtx, texture::{DepthTexture, Texture}};
use glam::{Mat4, mat4, uvec2, vec3, vec4};
use rustc_hash::{FxHashMap, FxHashSet};
use wgpu::{Origin3d, RenderPassDepthStencilAttachment, TexelCopyTextureInfo, wgt::CommandEncoderDescriptor};

use crate::{BLOCK_ATLAS_PNG_DIRT_SECTION, BLOCK_ATLAS_PNG_HEIGHT, BLOCK_ATLAS_PNG_WIDTH, GENERATED_BLOCK_ATLAS_PNG, GENERATED_CRAFTING_RECIPES_JSON, GENERATED_ITEM_ATLAS_PNG, ITEM_ATLAS_PNG_FARTHEST_SECTION, RES_SHADERS_BLOCK_RENDERER_WGSL, block_models::block_model, block_states::BlockStateProvider, blocks::{Block, BlockID, NOT_RENDERED_LAYER, }, const_block_model_types::Vertex, crafting::{CraftingRecipe, CraftingRecipeJson}, game::ScreenInfo, inventory::ItemStack, items::{self, BlockItem, Item, ItemID, ItemIDRep, ItemProperties}};

pub struct ItemAtlasRegistry {
    pub texture: UniformBinding<Texture>,
    depth_texture: DepthTexture,
    item_sections: FxHashMap<ItemIDRep, AtlasSection>,
    most_recent_section: AtlasSection,
    block_renderer_screen_info: UniformBinding<ScreenInfo>,
    block_renderer_transform_matrix: UniformBinding<[[f32; 4]; 4]>,
    rendered_item_resolution: u32,
}

impl ItemAtlasRegistry {
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let texture = UniformBinding::new(surface_ctx.device(), "Item Atlas", Texture::from_bytes(surface_ctx, &GENERATED_ITEM_ATLAS_PNG.load(), "Item Atlas", None, None).unwrap(), None);
        let block_renderer_screen_info = UniformBinding::new(surface_ctx.device(), "", ScreenInfo { 
            camera_raw: OrthographicCamera {
                eye: vec3(0.0, 1.0, 0.0),
                target: vec3(1.0, 0.0, 1.0),
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
            item_sections: FxHashMap::default(),
            most_recent_section: ITEM_ATLAS_PNG_FARTHEST_SECTION,
            rendered_item_resolution: 64,
            depth_texture: DepthTexture::create_depth_texture(surface_ctx.device(), texture.value.size.width, texture.value.size.height, "Item Atlas Depth Texture", 1),
            texture,
            block_renderer_screen_info,
            block_renderer_transform_matrix,
        }
    }

    // pub fn items_per_row(&self) -> u32 {
    //     self.texture.value.size.width/self.item_render_size
    // }

    // pub fn num_rows(&self) -> u32 {
    //     self.texture.value.size.height/self.item_render_size
    // }

    // pub fn fractional_item_size(&self) -> Vector2<f32> {
    //     vec2(1.0/self.items_per_row() as f32, 1.0/self.num_rows() as f32)
    // }

    pub fn get_item(&self, item: impl Into<ItemIDRep>) -> AtlasSection {
        let item = item.into();
        if let Some(section) = self.item_sections.get(&item) {
            return *section;
        } else {
            panic!("no atlas entry for item {item:?}");
        }
    }

    pub fn fractional_atlas_section(&self, section: &AtlasSection) -> [f32; 4] {
        [section.x as f32/self.texture.value.size.width as f32, section.y as f32/self.texture.value.size.height as f32, section.width as f32/self.texture.value.size.width as f32, section.height as f32/self.texture.value.size.height as f32]
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
    ) -> Mat4 {
        let scale_x = width;
        let scale_y = height;

        let center_u = start_x + width * 0.5;
        let center_v = start_y + height * 0.5;

        let translate_x = center_u * 2.0 - 1.0;
        let translate_y = 1.0 - center_v * 2.0;

        let m = mat4(
            vec4(scale_x, 0.0,     0.0, 0.0),
            vec4(0.0,     scale_y, 0.0, 0.0),
            vec4(0.0,     0.0,     1.0, 0.0),
            vec4(translate_x, translate_y, 0.0, 1.0),
        );
        m
    }

    pub fn resize_atlas(&mut self, to_width: u32, to_height: u32, surface_ctx: &dyn SurfaceCtx) {
        let new_texture = UniformBinding::new(surface_ctx.device(), "Item Atlas", Texture::blank_texture(surface_ctx.device(), to_width, to_height, self.texture.value.format, self.texture.value.texture.sample_count(), None, None), None);
        let new_depth_texture = DepthTexture::create_depth_texture(surface_ctx.device(), to_width, to_height, "Item Atlas Depth Texture", self.depth_texture.view.texture().sample_count());
        let mut encoder = surface_ctx.device().create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_texture_to_texture(TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            mip_level: 0,
            origin: Origin3d::ZERO,
            texture: &self.texture.value.texture,
        }, TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            mip_level: 0,
            origin: Origin3d::ZERO,
            texture: &new_texture.value.texture,
        }, self.texture.value.texture.size());
        surface_ctx.queue().submit([encoder.finish()]);
        self.texture = new_texture;
        self.depth_texture = new_depth_texture;
    }
}

pub struct Registries {
    pub block_atlas_texture: UniformBinding<Texture>,
    pub item_atlas_registry: ItemAtlasRegistry,
    pub item_registry: ItemRegistry,
    pub block_registry: BlockRegistry,
    pub crafting_registry: CraftingRecipeRegistry,
    pub tag_registry: TagRegistry,
    pub block_state_registry: BlockStateRegistry,
}

impl Registries {
    pub fn new(surface_ctx: &dyn SurfaceCtx) -> Self {
        let block_atlas_texture = UniformBinding::new(surface_ctx.device(), "Block Atlas", Texture::from_bytes(surface_ctx, &GENERATED_BLOCK_ATLAS_PNG.load(), "Block Atlas", None, None).unwrap(), None);
        let item_atlas_registry = ItemAtlasRegistry::new(surface_ctx);
        let mut item_registry = ItemRegistry::new();
        let mut block_registry = BlockRegistry::new();
        let mut crafting_registry = CraftingRecipeRegistry::new();
        let mut tag_registry = TagRegistry::new();
        let mut block_state_registry = BlockStateRegistry::new();

        block_registry.register_all();
        item_registry.register_all();
        for block in block_registry.blocks.values() {
            if let Some(item_id) = block.item {
                item_registry.register(Item { id: item_id, properties: ItemProperties::BlockItem(BlockItem { block: block.id }), attribute_modifiers: Default::default() });
            }
        }
        tag_registry.register_all(&item_registry);
        crafting_registry.register_all(&item_registry, &tag_registry);
        block_state_registry.register_all();

        let mut _self = Self {
            block_atlas_texture,
            item_atlas_registry,
            item_registry,
            block_registry,
            crafting_registry,
            tag_registry,
            block_state_registry,
        };

        
        _self.insert_items_in_atlas(surface_ctx);

        _self
    }

    fn insert_items_in_atlas(&mut self, surface_ctx: &dyn SurfaceCtx) {
        let shader_atlas_x_blocks = ResourceConst { name: "ATLAS_X_BLOCKS".into(), rtype: "u32".into(), value: (BLOCK_ATLAS_PNG_WIDTH / BLOCK_ATLAS_PNG_DIRT_SECTION.width).to_string() };
        let shader_atlas_y_blocks  = ResourceConst { name: "ATLAS_Y_BLOCKS".into(), rtype: "u32".into(), value: (BLOCK_ATLAS_PNG_HEIGHT / BLOCK_ATLAS_PNG_DIRT_SECTION.height).to_string() };
        let block_renderer_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_BLOCK_RENDERER_WGSL, formats: vec![self.item_atlas_registry.texture.value.format], uniforms: vec![&self.item_atlas_registry.block_renderer_screen_info, &self.block_atlas_texture, &self.item_atlas_registry.block_renderer_transform_matrix], shader_consts: vec![shader_atlas_x_blocks, shader_atlas_y_blocks], vertex_buffers: vec![Vertex::desc()], multisample_count: 1, ..Default::default() }, surface_ctx.device());
        for item in self.item_registry.items.values() {
            let item_atlas_registry = &mut self.item_atlas_registry;
            let last_section = item_atlas_registry.most_recent_section;
            let needed_dimensions = match &item.properties {
                ItemProperties::BlockItem(_block_item) => {
                    println!("added {} as block item", item.id);
                    uvec2(item_atlas_registry.rendered_item_resolution, item_atlas_registry.rendered_item_resolution)
                },
                ItemProperties::BasicItem(basic_item) => {
                    println!("added {} as basic item", item.id);
                    item_atlas_registry.item_sections.insert(item.id.into(), basic_item.section);
                    continue;
                },
                ItemProperties::Nothing => {
                    println!("added {} as nothing", item.id);
                    item_atlas_registry.item_sections.insert(item.id.into(), AtlasSection { x: 0, y: 0, width: 0, height: 0 });
                    continue;
                }
            };
            let next_section = if last_section.x+last_section.width+needed_dimensions.x > item_atlas_registry.texture.value.size.width {
                AtlasSection { x: 0, y: last_section.y+last_section.height, width: needed_dimensions.x, height: needed_dimensions.y }
            } else {
                AtlasSection { x: last_section.x+last_section.width, y: last_section.y, width: needed_dimensions.x, height: needed_dimensions.y }
            };
            item_atlas_registry.most_recent_section = next_section;
            if next_section.x+next_section.width > item_atlas_registry.texture.value.size.width || next_section.y+next_section.height > item_atlas_registry.texture.value.size.height {
                item_atlas_registry.resize_atlas((next_section.x+next_section.width).max(item_atlas_registry.texture.value.size.width), (next_section.y+next_section.height).max(item_atlas_registry.texture.value.size.height), surface_ctx);
            }
            let fractional_section = item_atlas_registry.fractional_atlas_section(&next_section);
            match &item.properties {
                ItemProperties::BlockItem(block_item) => {
                    item_atlas_registry.block_renderer_transform_matrix.set_data(surface_ctx.queue(), ItemAtlasRegistry::viewport_subsection_matrix(fractional_section[0], fractional_section[1], fractional_section[2], fractional_section[3]).to_cols_array_2d());
                    item_atlas_registry.render_block(surface_ctx, self.block_registry.get_block(&block_item.block), &self.block_atlas_texture, &block_renderer_shader);
                    item_atlas_registry.item_sections.insert(item.id.into(), next_section);
                },
                ItemProperties::BasicItem(_) => {
                    continue;
                }
                ItemProperties::Nothing => {
                    continue;
                },
            }
            
        }
    }
}

pub struct CraftingRecipeRegistry {
    recipes: FxHashMap<String, Vec<CraftingRecipe>>,
}

impl CraftingRecipeRegistry {
    pub fn new() -> Self {
        Self {
            recipes: FxHashMap::default(),
        }
    }

    pub fn register_all(&mut self, item_registry: &ItemRegistry, tag_registry: &TagRegistry) {
        let recipes_string = GENERATED_CRAFTING_RECIPES_JSON.load_string();
        let recipes_json: Vec<CraftingRecipeJson> = serde_json::from_str(&recipes_string).unwrap();
        for recipe_json in recipes_json {
            self.register(CraftingRecipe::from_json(recipe_json, item_registry, tag_registry));
        }
    }

    pub fn register(&mut self, recipe: CraftingRecipe) {
        // let ingredient_ids: Vec<CraftingIngredientID> = recipe.pattern.iter().flatten().map(|stack| stack.ingredient.id()).unique().collect();
        if !self.recipes.contains_key(&recipe.crafting_type) {
            self.recipes.insert(recipe.crafting_type.clone(), vec![]);
        }
        self.recipes.get_mut(&recipe.crafting_type).unwrap().push(recipe);
    }

    pub fn get_recipe_for_pattern(&self, pattern: Vec<Vec<ItemStack>>, crafting_type: impl Into<String>, registries: &Registries) -> Option<CraftingRecipe> {
        let crafting_type: String = crafting_type.into();
        let shrunk_pattern = CraftingRecipe::shrink_pattern(pattern);
        for possible_match in self.recipes.get(&crafting_type).unwrap_or(&vec![]) {
            if possible_match.matches(&shrunk_pattern, registries) {
                return Some(possible_match.clone());
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
        let id = id.into();
        self.items.get(&id).cloned().unwrap_or_else(|| { println!("asked for invalid item {id}"); self.get_item(items::NOTHING) })
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

    pub fn register(&mut self, block: Block) {
        self.blocks.insert(block.id, block);
    }

    pub fn get_block(&self, id: &BlockID) -> Block {
        *self.blocks.get(id).unwrap()
    }
}

pub type TagID = &'static str;

pub struct TagRegistry {
    block_tags: FxHashMap<String, Tag<BlockID>>,
    item_tags: FxHashMap<String, Tag<ItemID>>,
}

pub struct Tag<ID> {
    pub id: TagID,
    pub entries: FxHashSet<ID>
}

impl TagRegistry {
    pub fn new() -> Self {
        Self {
            block_tags: FxHashMap::default(),
            item_tags: FxHashMap::default()
        }
    }

    pub fn register_block_tag(&mut self, id: &'static str, json: String) {
        self.block_tags.insert(id.into(), Tag { id, entries: serde_json::from_str(&json).unwrap() });
    }

    pub fn get_block_tag(&self, id: impl Into<String>) -> &Tag<BlockID> {
        let id = id.into();
        self.block_tags.get(&id).expect(&format!("no tag for {id}"))
    }

    pub fn register_item_tag(&mut self, id: &'static str, json: String, item_registry: &ItemRegistry) {
        let string_version: Vec<String> = serde_json::from_str(&json).unwrap();
        self.item_tags.insert(id.into(), Tag { id, entries: FxHashSet::from_iter(string_version.iter().map(|it| item_registry.get_item(it).id)) });
    }

    pub fn get_item_tag(&self, id: impl Into<String>) -> &Tag<ItemID> {
        let id = id.into();
        self.item_tags.get(&id).expect(&format!("no tag for {id}"))
    }

    // fn parse_json<'a, T: Deserialize<'a> + Eq + Hash>(json: String) -> FxHashSet<T> {
    //     let entries: FxHashSet<T> = ;
    //     FxHashSet::from_iter(entries)
    // }
}

pub struct BlockStateRegistry {
    entries: FxHashMap<BlockID, &'static dyn BlockStateProvider>
}

impl BlockStateRegistry {
    pub fn new() -> Self {
        Self {
            entries: FxHashMap::default()
        }
    }

    pub fn register(&mut self, block: BlockID, provider: &'static dyn BlockStateProvider) {
        self.entries.insert(block, provider);
    }

    pub fn get_state_provider(&self, block: BlockID) -> Option<&'static dyn BlockStateProvider> {
        self.entries.get(&block).cloned()
    }
}