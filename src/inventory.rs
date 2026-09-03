use std::collections::HashMap;

use bespoke_engine::{binding::{Descriptor, UniformBinding}, camera::OrthographicCamera, model::Render, resource_loader::ResourceConst, shader::{Shader, UniformShaderInit}, surface_context::SurfaceCtx, texture::{DepthTexture, Texture}};
use cgmath::{Matrix4, Vector2, vec2, vec3};
use wgpu::RenderPassDepthStencilAttachment;

use crate::{RES_SHADERS_BLOCK_RENDERER_WGSL, block_models::block_model, blocks::{self, Block, NOT_RENDERED_LAYER}, game::{ScreenInfo, Vertex}, items::{self, BlockItem, Item, ItemId, ItemProperties}};

#[derive(PartialEq, Eq, Debug)]
pub struct Inventory {
    pub items: Vec<InventoryItemStack>,
    pub selected: usize,
}

impl Inventory {
    pub fn empty_size(size: usize, item_atlas: &mut ItemAtlas, block_atlas: &UniformBinding<Texture>, surface_ctx: &dyn SurfaceCtx) -> Self {
        Self {
            items: vec![InventoryItemStack::new(ItemStack::EMPTY, item_atlas, block_atlas, surface_ctx); size],
            selected: 0,
        }
    }

    pub fn selected_item(&self) -> &InventoryItemStack {
        &self.items[self.selected]
    }

    pub fn selected_item_mut(&mut self) -> &mut InventoryItemStack {
        &mut self.items[self.selected]
    }

    pub fn add(&mut self, item_stack: &InventoryItemStack) -> bool {
        for inventory_stack in &mut self.items {
            if inventory_stack.stack.item == item_stack.stack.item {
                inventory_stack.stack.count += item_stack.stack.count;
                return true;
            }
            if inventory_stack.stack.is_empty() {
                *inventory_stack = item_stack.clone();
                return true;
            }
        }
        return false;
    }

    pub fn calculate_crafting_result(&mut self) {

    }

    pub fn take_crafting_result(&mut self, cursor_stack: &mut InventoryItemStack) {
        
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InventoryItemStack {
    pub stack: ItemStack,
    pub atlas_coordinates: Vector2<u32>,
}

impl InventoryItemStack {
    pub fn new(stack: ItemStack, item_atlas: &mut ItemAtlas, block_atlas: &UniformBinding<Texture>, surface_ctx: &dyn SurfaceCtx) -> Self {
        Self {
            atlas_coordinates: item_atlas.get_or_insert_item(&stack.item, block_atlas, surface_ctx),
            stack,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ItemStack {
    pub item: Item,
    pub count: i32,
}

impl ItemStack {
    pub const EMPTY: Self = ItemStack {
        item: items::NOTHING,
        count: 0,
    };

    pub fn new(item: Item, count: i32) -> Self {
        Self {
            count,
            item,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.item == items::NOTHING || self.item.properties == ItemProperties::BlockItem(BlockItem { block: blocks::AIR })
    }
}

pub struct ItemAtlas<'a> {
    pub texture: UniformBinding<Texture>,
    depth_texture: DepthTexture,
    item_positions: HashMap<ItemId, Vector2<u32>>,
    block_renderer_shader: Shader<'a>,
    block_renderer_screen_info: UniformBinding<ScreenInfo>,
    block_renderer_transform_matrix: UniformBinding<[[f32; 4]; 4]>,
    item_render_size: u32,
}

impl <'a> ItemAtlas<'a> {
    pub fn new(surface_ctx: &dyn SurfaceCtx, block_atlas: &UniformBinding<Texture>, shader_consts: Vec<ResourceConst>) -> Self {
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
        let block_renderer_shader = Shader::new(UniformShaderInit { resource: RES_SHADERS_BLOCK_RENDERER_WGSL, formats: vec![surface_ctx.config().format], uniforms: vec![&block_renderer_screen_info, block_atlas, &block_renderer_transform_matrix], shader_consts, vertex_buffers: vec![Vertex::desc()], multisample_count: 1, ..Default::default() }, surface_ctx.device());
        Self {
            item_positions: HashMap::new(),
            item_render_size: 64,
            depth_texture: DepthTexture::create_depth_texture(surface_ctx.device(), texture.value.size.width, texture.value.size.height, "Item Atlas Depth Texture", 1),
            texture,
            block_renderer_screen_info,
            block_renderer_shader,
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

    pub fn get_or_insert_item(&mut self, item: &Item, block_atlas: &UniformBinding<Texture>, surface_ctx: &dyn SurfaceCtx) -> Vector2<u32> {
        if let Some(position) = self.item_positions.get(item.id) {
            return vec2(position.x, position.y);
        } else {
            let next_position = vec2((self.item_positions.len()) as u32 %self.items_per_row(), (self.item_positions.len()) as u32 /self.items_per_row());
            let fractional_position = vec2(next_position.x as f32 / self.items_per_row() as f32, next_position.y as f32 / self.num_rows() as f32);
            self.block_renderer_transform_matrix.set_data(surface_ctx.queue(), Self::viewport_subsection_matrix(fractional_position.x, fractional_position.y, self.fractional_item_size().x, self.fractional_item_size().y).into());
            match &item.properties {
                ItemProperties::BlockItem(block_item) => {
                    self.render_block(surface_ctx, block_item.block, block_atlas);
                },
                ItemProperties::Nothing => {},
            }
            self.item_positions.insert(item.id, next_position);
            return next_position;
        }
    }

    pub fn subsection_for_position(&self, position: Vector2<u32>) -> [f32; 4] {
        [position.x as f32/self.items_per_row() as f32, position.y as f32/self.num_rows() as f32, self.fractional_item_size().x, self.fractional_item_size().y]
    }

    pub fn render_block(&self, surface_ctx: &dyn SurfaceCtx, block: Block, block_atlas: &UniformBinding<Texture>) {
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
            self.block_renderer_shader.bind(&mut render_pass);
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