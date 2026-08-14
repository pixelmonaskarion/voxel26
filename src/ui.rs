use bespoke_engine::{binding::Descriptor, culling::AABB, model::{Model, ToRaw}, surface_context::SurfaceCtx};
use bytemuck::{NoUninit, bytes_of};
use cgmath::{Vector2, vec2};
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};

use crate::{blocks::{ATLAS_X_BLOCKS, ATLAS_Y_BLOCKS}, inventory::ItemStack};

#[repr(C)]
#[derive(NoUninit, Copy, Clone, Default, Debug)]
pub struct UIVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub repeat_count: [f32; 2],
}

impl Descriptor for UIVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl ToRaw for UIVertex {
    fn to_raw(&self) -> Vec<u8> {
        bytes_of(self).to_vec()
    }
}

pub fn generate_hotbar_item_ui_models(surface_ctx: &dyn SurfaceCtx) -> Vec<Model> {
    let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
    let item_ui_height = hotbar_item_height();
    let item_ui_width = item_ui_height / aspect_ratio;
    (0..9).map(|it| {
        Model::new(vec![
            UIVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0, 0.0], tex_coords: [0.0, 1.0], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [0.0, 0.0], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0, 0.0], tex_coords: [1.0, 1.0], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [1.0, 0.0], repeat_count: [1.0, 1.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device())
    }).collect()
}

pub fn generate_health_ui_models(surface_ctx: &dyn SurfaceCtx, health: f32) -> Vec<Model> {
    let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
    let item_ui_height = hotbar_item_height();
    let item_ui_width = item_ui_height / aspect_ratio;
    let heart_height = hotbar_item_height()*0.4;
    let heart_width = heart_height / aspect_ratio;
    let full_coords = [3.0, 0.0];
    let halve_coords = [4.0, 0.0];
    let none_coords = [5.0, 0.0];
    (0..10).map(|it| {
        let coords = if (it as f32) < (health-1.0)/2.0 {
            full_coords
        } else {
            if (it as f32) < health/2.0 {
                halve_coords
            } else {
                none_coords
            }
        };
        Model::new(vec![
            UIVertex { position: [(it as f32)*heart_width-item_ui_width*4.5, -1.0+item_ui_height, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32)*heart_width-item_ui_width*4.5, -1.0+item_ui_height+heart_height, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 + 1.0)*heart_width-item_ui_width*4.5, -1.0+item_ui_height, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 + 1.0)*heart_width-item_ui_width*4.5, -1.0+item_ui_height+heart_height, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device())
    }).collect()
}

pub fn hotbar_item_height() -> f32 {
    return 0.2;
}

pub fn generate_hotbar_background_ui_models(surface_ctx: &dyn SurfaceCtx, selected: usize) -> Vec<Model> {
    let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
    let item_ui_height = 0.2;
    let item_ui_width = 0.2 / aspect_ratio;
    let selected_coords = [1.0, 0.0];
    let unselected_coords = [0.0, 0.0];
    (0..9).map(|it| {
        let coords = if it == selected {
            selected_coords
        } else {
            unselected_coords
        };
        Model::new(vec![
            UIVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
            UIVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device())
    }).collect()
}

pub fn generate_crosshair_ui_model(surface_ctx: &dyn SurfaceCtx) -> Model {
    let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
    let height = 0.1;
    let width = 0.1 / aspect_ratio;
    let coords = [2.0, 0.0];
    Model::new(vec![
        UIVertex { position: [-width/2.0, -height/2.0, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
        UIVertex { position: [-width/2.0, height/2.0, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
        UIVertex { position: [width/2.0, -height/2.0, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
        UIVertex { position: [width/2.0, height/2.0, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32], repeat_count: [1.0, 1.0] },
    ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device())
}

#[derive(PartialEq, Eq)]
pub enum OpenInventory {
    PlayerInventory,
}

pub struct InventoryModel {
    pub container: Model,
    pub items: Vec<Model>,
}

pub fn inventory_margins(inventory: &OpenInventory) -> f32 {
    match inventory {
        OpenInventory::PlayerInventory => 0.1
    }
}

pub fn inventory_size(inventory: &OpenInventory) -> (i32, i32) {
    match inventory {
        OpenInventory::PlayerInventory => (4, 9)
    }
}

pub fn create_inventory_model(surface_ctx: &dyn SurfaceCtx, inventory: &OpenInventory) -> InventoryModel {
    let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
    let margins = inventory_margins(inventory);
    let (margin_width, margin_height) = if surface_ctx.config().width > surface_ctx.config().height {
        (margins / aspect_ratio, margins)
    } else {
        (margins, margins * aspect_ratio)
    };
    let (num_rows, num_cols) = inventory_size(inventory);
    let (tile_width, tile_height) = if num_rows > num_cols {
        let height = (2.0-margin_height*2.0)/num_rows as f32;
        (height / aspect_ratio, height)
    } else {
        let width = (2.0-margin_width*2.0)/num_cols as f32;
        (width, width * aspect_ratio)
    };
    let total_width = num_cols as f32 * tile_width;
    let total_height = num_rows as f32 * tile_height;
    let mut vertices: Vec<UIVertex> = vec![];
    let mut indices: Vec<u16> = vec![];
    let mut items = vec![];
    //0, 0 is top left
    let mut add_quad = |start: Vector2<f32>, end: Vector2<f32>, rw: f32, rh: f32, ax: f32, ay: f32| {
        let start = vec2(start.x-total_width/2.0, -start.y+total_height/2.0);
        let end = vec2(end.x-total_width/2.0, -end.y+total_height/2.0);
        let start_index = vertices.len() as u16;
        vertices.extend_from_slice(&[
            UIVertex { position: [start.x, end.y, 0.0], tex_coords: [ax/ATLAS_X_BLOCKS as f32, (ay+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [rw, rh] },
            UIVertex { position: [start.x, start.y, 0.0], tex_coords: [ax/ATLAS_X_BLOCKS as f32, ay/ATLAS_Y_BLOCKS as f32], repeat_count: [rw, rh] },
            UIVertex { position: [end.x, end.y, 0.0], tex_coords: [(ax+1.0)/ATLAS_X_BLOCKS as f32, (ay+1.0)/ATLAS_Y_BLOCKS as f32], repeat_count: [rw, rh] },
            UIVertex { position: [end.x, start.y, 0.0], tex_coords: [(ax+1.0)/ATLAS_X_BLOCKS as f32, ay/ATLAS_Y_BLOCKS as f32], repeat_count: [rw, rh] },
        ]);
        indices.extend_from_slice(&[0+start_index, 2+start_index, 1+start_index, 2+start_index, 3+start_index, 1+start_index]);
    };
    let mut add_item = |start: Vector2<f32>, end: Vector2<f32>, rw: f32, rh: f32| {
        let start = vec2(start.x-total_width/2.0, -start.y+total_height/2.0);
        let end = vec2(end.x-total_width/2.0, -end.y+total_height/2.0);
        items.push(Model::new(vec![
            UIVertex { position: [start.x, end.y, 0.0], tex_coords: [0.0, 1.0], repeat_count: [rw, rh] },
            UIVertex { position: [start.x, start.y, 0.0], tex_coords: [0.0, 0.0], repeat_count: [rw, rh] },
            UIVertex { position: [end.x, end.y, 0.0], tex_coords: [1.0, 1.0], repeat_count: [rw, rh] },
            UIVertex { position: [end.x, start.y, 0.0], tex_coords: [1.0, 0.0], repeat_count: [rw, rh] },
        ], &[0u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device()));
    };
    
    add_quad(vec2(0.0, 0.0), vec2(tile_width, tile_height), 1.0, 1.0, 0.0, 1.0);
    add_quad(vec2(tile_width*(num_cols as f32 - 1.0), 0.0), vec2(tile_width*(num_cols as f32), tile_height), 1.0, 1.0, 2.0, 1.0);
    add_quad(vec2(0.0, tile_height*(num_rows as f32 - 1.0)), vec2(tile_width, tile_height*(num_rows as f32)), 1.0, 1.0, 0.0, 3.0);
    add_quad(vec2(tile_width*(num_cols as f32 - 1.0), tile_height*(num_rows as f32 - 1.0)), vec2(tile_width*(num_cols as f32), tile_height*(num_rows as f32)), 1.0, 1.0, 2.0, 3.0);
    if num_cols > 2 {
        add_quad(vec2(tile_width, 0.0), vec2(tile_width*(num_cols as f32 - 1.0), tile_height), num_cols as f32 - 2.0, 1.0, 1.0, 1.0);
        add_quad(vec2(tile_width, tile_height*(num_rows as f32 - 1.0)), vec2(tile_width*(num_cols as f32 - 1.0), tile_height*(num_rows as f32)), num_cols as f32 - 2.0, 1.0, 1.0, 3.0);
    }
    if num_rows > 2 {
        add_quad(vec2(0.0, tile_height), vec2(tile_width, tile_height*(num_rows as f32-1.0)), 1.0, num_rows as f32 - 2.0, 0.0, 2.0);
        add_quad(vec2(tile_width*(num_cols as f32 - 1.0), tile_height), vec2(tile_width+tile_width*(num_cols as f32 - 1.0), tile_height*(num_rows as f32-1.0)), 1.0, num_rows as f32 - 2.0, 2.0, 2.0);
    }
    if num_cols > 2 && num_rows > 2 {
        add_quad(vec2(tile_width, tile_height), vec2(tile_width*(num_cols as f32 - 1.0), tile_height*(num_rows as f32 - 1.0)), num_cols as f32 - 2.0, num_rows as f32 - 2.0, 1.0, 2.0);
    }
    for y in 0..num_rows {
        for x in 0..num_cols {
            add_quad(vec2(tile_width * x as f32, tile_height * y as f32), vec2(tile_width * x as f32 + tile_width, tile_height * y as f32 + tile_height), 1.0, 1.0, 1.0, 0.0);
            add_item(vec2(tile_width * x as f32, tile_height * y as f32), vec2(tile_width * x as f32 + tile_width, tile_height * y as f32 + tile_height), 1.0, 1.0);
        }
    }
    InventoryModel { container:  Model::new(vertices, &indices, AABB::zero(), surface_ctx.device()), items }
}

pub fn text_sections_for_inventory<'a>(surface_ctx: &dyn SurfaceCtx, item_at: &'a dyn Fn(usize) -> &'a ItemStack, inventory: &OpenInventory) -> Vec<OwnedSection> {
    let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
    let margins = inventory_margins(inventory);
    let (margin_width, margin_height) = if surface_ctx.config().width > surface_ctx.config().height {
        (margins / aspect_ratio, margins)
    } else {
        (margins, margins * aspect_ratio)
    };
    let (num_rows, num_cols) = inventory_size(inventory);
    let (tile_width, tile_height) = if num_rows > num_cols {
        let height = (2.0-margin_height*2.0)/num_rows as f32;
        (height / aspect_ratio, height)
    } else {
        let width = (2.0-margin_width*2.0)/num_cols as f32;
        (width, width * aspect_ratio)
    };
    let total_width = num_cols as f32 * tile_width;
    let total_height = num_rows as f32 * tile_height;
    let mut sections = vec![];
    for y in 0..num_rows {
        for x in 0..num_cols {
            let count = item_at((y * num_cols + x) as usize).count;
            if count > 1 {
                sections.push(OwnedSection::default()
                    .add_text(
                        OwnedText::new(format!("{}", count))
                        .with_scale(surface_ctx.config().height as f32 * tile_height/4.0)
                        .with_color([0.0, 0.0, 0.0, 1.0]))
                    .with_bounds((tile_width * surface_ctx.config().width as f32, tile_height * surface_ctx.config().height as f32))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Right).v_align(VerticalAlign::Bottom))
                    .with_screen_position((((tile_width * (x as f32 + 1.0) - tile_width/8.0 -total_width/2.0)/2.0 + 0.5)*surface_ctx.config().width as f32, ((-tile_height * (y as f32 + 1.0) +tile_height/16.0 +total_height/2.0)/-2.0 + 0.5)*surface_ctx.config().height as f32))
                );
            }
        }
    }
    sections
}

pub fn mouse_tile_coords(mouse_coords: Vector2<f32>, surface_ctx: &dyn SurfaceCtx, inventory: &OpenInventory) -> Option<usize> {
    let screen_coords = vec2(mouse_coords.x*2.0 - 1.0, -mouse_coords.y*2.0 + 1.0);
    let aspect_ratio = surface_ctx.config().width as f32 / surface_ctx.config().height as f32;
    let margins = inventory_margins(inventory);
    let (margin_width, margin_height) = if surface_ctx.config().width > surface_ctx.config().height {
        (margins / aspect_ratio, margins)
    } else {
        (margins, margins * aspect_ratio)
    };
    let (num_rows, num_cols) = inventory_size(inventory);
    let (tile_width, tile_height) = if num_rows > num_cols {
        let height = (2.0-margin_height*2.0)/num_rows as f32;
        (height / aspect_ratio, height)
    } else {
        let width = (2.0-margin_width*2.0)/num_cols as f32;
        (width, width * aspect_ratio)
    };
    let total_width = num_cols as f32 * tile_width;
    let total_height = num_rows as f32 * tile_height;
    for y in 0..num_rows {
        for x in 0..num_cols {
            let start = vec2(tile_width * x as f32, tile_height * y as f32);
            let end = vec2(tile_width * x as f32 + tile_width, tile_height * y as f32 + tile_height);
            let start = vec2(start.x-total_width/2.0, -start.y+total_height/2.0);
            let end = vec2(end.x-total_width/2.0, -end.y+total_height/2.0);
            if screen_coords.x >= start.x && screen_coords.x < end.x && screen_coords.y < start.y && screen_coords.y >= end.y {
                return Some((y * num_cols + x) as usize);
            }
        }
    }
    None
}