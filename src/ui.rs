use bespoke_engine::{culling::AABB, model::Model, surface_context::SurfaceCtx, window::BasicVertex};

use crate::blocks::{ATLAS_X_BLOCKS, ATLAS_Y_BLOCKS};


pub fn generate_hotbar_item_ui_models(surface_ctx: &dyn SurfaceCtx, aspect_ratio: f32) -> Vec<Model> {
    let item_ui_height = 0.2;
    let item_ui_width = 0.2 / aspect_ratio;
    (0..9).map(|it| {
        Model::new(vec![
            BasicVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0, 0.0], tex_coords: [0.0, 1.0] },
            BasicVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [0.0, 0.0] },
            BasicVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0, 0.0], tex_coords: [1.0, 1.0] },
            BasicVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [1.0, 0.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device())
    }).collect()
}

pub fn generate_hotbar_background_ui_models(surface_ctx: &dyn SurfaceCtx, aspect_ratio: f32, selected: usize) -> Vec<Model> {
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
            BasicVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32] },
            BasicVertex { position: [(it as f32 - 4.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32] },
            BasicVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32] },
            BasicVertex { position: [(it as f32 - 3.5)*item_ui_width, -1.0+item_ui_height, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device())
    }).collect()
}

pub fn generate_crosshair_ui_model(surface_ctx: &dyn SurfaceCtx, aspect_ratio: f32) -> Model {
    let height = 0.1;
    let width = 0.1 / aspect_ratio;
    let coords = [2.0, 0.0];
    Model::new(vec![
        BasicVertex { position: [-width/2.0, -height/2.0, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32] },
        BasicVertex { position: [-width/2.0, height/2.0, 0.0], tex_coords: [coords[0]/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32] },
        BasicVertex { position: [width/2.0, -height/2.0, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, (coords[1]+1.0)/ATLAS_Y_BLOCKS as f32] },
        BasicVertex { position: [width/2.0, height/2.0, 0.0], tex_coords: [(coords[0]+1.0)/ATLAS_X_BLOCKS as f32, coords[1]/ATLAS_Y_BLOCKS as f32] },
    ], &[0_u16, 2, 1, 2, 3, 1], AABB::zero(), surface_ctx.device())
}