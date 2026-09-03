use std::path::Path;

use bespoke_engine::{resource_compiler::dir_contents, resource_loader::ResourceGenerator};
use image::{ColorType, ImageFormat};

fn main() {
    let mut resource_generator = ResourceGenerator::new(true);
    
    resource_generator.add_path(Path::new("src/res"));

    resource_generator.compile_atlas("block_atlas.png", &dir_contents("src/tbg-res/blocks"), ColorType::Rgba8, ImageFormat::Png, 512);
    resource_generator.compile_merged_json(&dir_contents("src/tbg-res/crafting_recipes"), "crafting_recipes.json");

    resource_generator.generate();
}