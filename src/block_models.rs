use bespoke_engine::{culling::AABB, model::Model, resource_loader::load_resource_string, surface_context::SurfaceCtx};
use cgmath::Vector3;
use serde::Deserialize;

use crate::{blocks::{ATLAS_X_BLOCKS, ATLAS_Y_BLOCKS, Block, BlockID}, game::Vertex};

pub fn block_model(surface_ctx: &dyn SurfaceCtx, block: Block) -> Model {
    let BlockModel { vertices, indices } = if block.has_model {
        parse_model(block.id)
    } else {
        let vertices = vec![
            // north
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
                normal: [0.0, 0.0, -1.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            // south
            Vertex {
                position: [0.0, 0.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            // east
            Vertex {
                position: [1.0, 0.0, 1.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 1.0, 1.0],
                normal: [1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            // west
            Vertex {
                position: [0.0, 0.0, 0.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 1.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [-1.0, 0.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            // up
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            // down
            Vertex {
                position: [1.0, 0.0, 1.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 1.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, block.atlas_y as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [block.atlas_x as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [0.0, -1.0, 0.0, 0.0],
                color: [(block.atlas_x + 1) as f32 / ATLAS_X_BLOCKS as f32, (block.atlas_y + 1) as f32 / ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
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

#[derive(Deserialize)]
struct ModelDefinition {
    elements: Vec<ModelElement>
}

#[derive(Deserialize)]
struct ModelElement {
    from: [i32; 3],
    to: [i32; 3],
    rotation: ModelRotation,
    faces: ModelFaces,
}

#[derive(Deserialize)]
struct ModelFaces {
    north: ModelFace,
    east: ModelFace,
    south: ModelFace,
    west: ModelFace,
    up: ModelFace,
    down: ModelFace,
}

#[derive(Deserialize)]
struct ModelFace {
    uv: [i32; 4],
    texture: String,
}

#[derive(Deserialize)]
struct ModelRotation {
    angle: f32,
    axis: String,
    origin: [f32; 3],
}

pub struct BlockModel {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub fn parse_model(id: BlockID) -> BlockModel {
    let definition_json = load_resource_string(&format!("res/models/{id}.json"));
    let definition: ModelDefinition = serde_json::from_str(&definition_json).unwrap();
    let mut vertices = vec![];
    let mut indices = vec![];

    for element in definition.elements {
        let from: Vector3<f32> = Vector3::from(element.from).cast().unwrap();
        let to: Vector3<f32> = Vector3::from(element.to).cast().unwrap();
        let scale = 1.0/16.0;

        //north
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [element.faces.north.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.north.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [element.faces.north.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.north.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [element.faces.north.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.north.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            color: [element.faces.north.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.north.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        }); 
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //south
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [element.faces.south.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.south.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [element.faces.south.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.south.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [element.faces.south.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.south.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            color: [element.faces.south.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.south.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //east
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [element.faces.east.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.east.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [element.faces.east.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.east.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [element.faces.east.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.east.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [1.0, 0.0, 0.0, 0.0],
            color: [element.faces.east.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.east.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //west
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [element.faces.west.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.west.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [element.faces.west.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.west.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [element.faces.west.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.west.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [-1.0, 0.0, 0.0, 0.0],
            color: [element.faces.west.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.west.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //up
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [element.faces.up.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.up.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, from.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [element.faces.up.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.up.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [element.faces.up.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.up.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, to.y * scale, to.z * scale, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            color: [element.faces.up.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.up.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);
        //down
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [element.faces.down.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.down.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, to.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [element.faces.down.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.down.uv[1] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [from.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [element.faces.down.uv[0] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.down.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [to.x * scale, from.y * scale, from.z * scale, 1.0],
            normal: [0.0, -1.0, 0.0, 0.0],
            color: [element.faces.down.uv[2] as f32 * scale/ATLAS_X_BLOCKS as f32, element.faces.down.uv[3] as f32 * scale/ATLAS_Y_BLOCKS as f32, 1.0, 1.0],
        });
        indices.extend_from_slice(&[
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 3,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 4,
            vertices.len() as u32 - 2,
            vertices.len() as u32 - 1,
        ]);

    }

    BlockModel { vertices, indices }
}