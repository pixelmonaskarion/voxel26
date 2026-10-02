use bespoke_engine::{culling::AABB, model::Model, surface_context::SurfaceCtx};

use crate::{BLOCK_ATLAS_PNG_HEIGHT, BLOCK_ATLAS_PNG_WIDTH, blocks::Block, const_block_model_types::{BlockModel, Vertex}};

pub fn block_model(surface_ctx: &dyn SurfaceCtx, block: Block) -> Model {
    let BlockModel { vertices, indices } = if let Some(model) = block.model {
        BlockModel { vertices: model.vertices().to_vec(), indices: model.indices().to_vec() }
    } else {
        let atlas_width_proportion = 1.0 / BLOCK_ATLAS_PNG_WIDTH as f32;
        let atlas_height_proportion = 1.0 / BLOCK_ATLAS_PNG_HEIGHT as f32;
        let vertices = vec![
            // north
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            // south
            Vertex {
                position: [0.0, 0.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            // east
            Vertex {
                position: [1.0, 0.0, 1.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 1.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            // west
            Vertex {
                position: [0.0, 0.0, 0.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 1.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            // up
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            // down
            Vertex {
                position: [1.0, 0.0, 1.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 1.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, block.atlas_section.y as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [block.atlas_section.x as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [(block.atlas_section.x + block.atlas_section.width) as f32 * atlas_width_proportion, (block.atlas_section.y + block.atlas_section.height) as f32 * atlas_height_proportion, 1.0, 1.0],
            },
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3,
            4, 5, 6, 4, 6, 7,
            8, 9, 10, 8, 10, 11,
            12, 13, 14, 12, 14, 15,
            16, 17, 18, 16, 18, 19,
            20, 21, 22, 20, 22, 23,
        ];
        BlockModel { vertices, indices }
    };
    Model::new(vertices, &indices, AABB::zero(), surface_ctx.device())
}