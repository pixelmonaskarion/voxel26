use std::path::Path;

use bespoke_engine::{resource_compiler::dir_contents, resource_loader::ResourceGenerator};
use image::{ColorType, ImageFormat};

include!("src/const_block_model_types.rs");
include!("src/const_block_models.rs");

fn main() {
    let mut resource_generator = ResourceGenerator::new(false);
    
    resource_generator.add_path(Path::new("src/res"));

    let block_atlas = resource_generator.compile_atlas("block_atlas.png", &dir_contents("src/tbg-res/textures/blocks"), ColorType::Rgba8, ImageFormat::Png, 512, 0);
    resource_generator.compile_atlas("item_atlas.png", &dir_contents("src/tbg-res/textures/items"), ColorType::Rgba8, ImageFormat::Png, 512, 512);
    resource_generator.compile_merged_json_list(&dir_contents("src/tbg-res/crafting_recipes"), "crafting_recipes.json");
    // resource_generator.compile_merged_json_map(&dir_contents("src/tbg-res/tags/blocks"), "block_tags.json");

    generate_models(&mut resource_generator, dir_contents("src/tbg-res/models"), &block_atlas);

    resource_generator.generate();
}