use std::path::Path;

use bespoke_engine::{resource_compiler::dir_contents, resource_loader::ResourceGenerator};
use image::{ColorType, ImageFormat};

fn main() {
    let mut resource_generator = ResourceGenerator::new(true);
    
    resource_generator.add_path(Path::new("src/res"));

    let mut blocks = dir_contents("src/tbg-res/blocks");
    blocks.sort_by_cached_key(|it| it.file_name().unwrap().to_string_lossy().to_string());
    resource_generator.compile_atlas("block_atlas.png", &blocks, ColorType::Rgba8, ImageFormat::Png, 512);

    resource_generator.generate();
}