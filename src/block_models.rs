use bespoke_engine::resource_loader::load_resource_string;
use cgmath::Vector3;
use serde::Deserialize;

use crate::{blocks::BlockID, game::Vertex};

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

pub fn parse_model(id: BlockID) -> (Vec<Vertex>, Vec<u32>) {
    let definition_json = load_resource_string(&format!("models/{id}.json"));
    let definition: ModelDefinition = serde_json::from_str(&definition_json).unwrap();
    let vertices = vec![];
    let indices = vec![];

    for element in definition.elements {
        let from: Vector3<f32> = Vector3::from(element.from).cast().unwrap();
        let to: Vector3<f32> = Vector3::from(element.to).cast().unwrap();
    }

    (vertices, indices)
}