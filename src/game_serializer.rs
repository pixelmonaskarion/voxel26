use std::{fs::{self, create_dir_all, read, write}, io::{self, Write}, num::ParseIntError, path::PathBuf, time::Duration};

use bespoke_engine::{camera::Camera, resource_compiler::dir_contents};
use cgmath::Vector3;
use directories::ProjectDirs;
use flate2::Compression;
use itertools::Itertools;
use rkyv::Archive;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{crafting::ItemStackData, game::Game, inventory::{Inventory, InventoryItemStack, ItemStack}, player::{EntityAttribute, Player}, registries::Registries};

pub struct GameSerializer {
    project_dirs: ProjectDirs,
    pub world_info: Option<WorldInfo>,
}

impl GameSerializer {
    pub fn new() -> Self {
        let mut _self = Self {
            project_dirs: ProjectDirs::from("com", "chrissytopher", "voxel26").unwrap(),
            world_info: None,
        };
        _self.get_world_info();
        _self
    }

    pub fn get_world_info(&mut self) {
        let data_dir = self.project_dirs.data_dir().to_path_buf();
        let world_info_file = data_dir.join("world_info.rmp");
        if world_info_file.exists() {
            if let Ok(Ok(world_info)) = read(world_info_file).map(|file| rkyv::access(&Self::ungzip(&file))) {
                self.world_info = Some(world_info);
            }
        }
    }

    pub fn save_world(&mut self, game: &mut Game) {
        // println!("DID NOT SAVE WORLD");
        // return;
        let data_dir = self.project_dirs.data_dir().to_path_buf();
        let world_dir = data_dir.join("world");
        create_dir_all(&world_dir).unwrap();
        //chunks
        for (pos, chunk) in game.chunk_manager.chunks() {
            if !chunk.generated_blocks {
                continue;
            }
            let chunk_file = world_dir.join(format!("x{}y{}z{}.rmp", pos[0], pos[1], pos[2]));
            write(chunk_file, Self::gzip(&rkyv::to_bytes(chunk).unwrap())).unwrap();
        }
        //player
        let player_file = data_dir.join("player.rmp");
        write(player_file, Self::gzip(&rkyv::to_bytes(&PlayerData::from_real(&game.player)).unwrap())).unwrap();
        //world_info
        let player_file = data_dir.join("world_info.rmp");
        write(player_file, Self::gzip(&rkyv::to_bytes(&game.world_info).unwrap())).unwrap();
    }

    pub fn load_world(&mut self, game: &mut Game) {
        let data_dir = self.project_dirs.data_dir().to_path_buf();
        let world_dir = data_dir.join("world");
        let player_file = data_dir.join("player.rmp");
        if world_dir.exists() && player_file.exists() {
            let player_json: PlayerData = rkyv::access(&Self::ungzip(&fs::read(player_file).unwrap())).unwrap();
            game.player = player_json.to_real(&game.registries);
            for chunk_file in dir_contents(world_dir) {
                if let Err(e) = self.load_chunk(game, chunk_file) {
                    println!("{e:?}");
                }
            }
        }
    }

    pub fn load_chunk(&mut self, game: &mut Game, chunk_file: PathBuf) -> Result<(), ChunkLoadError> {
        let name = chunk_file.file_prefix().unwrap().to_str().unwrap();
        let x_index = name.chars().find_position(|it| it == &'x').ok_or(ChunkLoadError::InvalidFileName(name.into()))?.0;
        let y_index = name.chars().find_position(|it| it == &'y').ok_or(ChunkLoadError::InvalidFileName(name.into()))?.0;
        let z_index = name.chars().find_position(|it| it == &'z').ok_or(ChunkLoadError::InvalidFileName(name.into()))?.0;
        let x = name[x_index+1..y_index].parse()?;
        let y = name[y_index+1..z_index].parse()?;
        let z = name[z_index+1..].parse()?;
        let pos = [x, y, z];
        let chunk_bytes = Self::ungzip(&read(chunk_file).unwrap());
        let chunk = rkyv::access(&chunk_bytes).unwrap();
        game.chunk_manager.replace_chunk(pos, chunk);
        Ok(())
    }

    pub fn ungzip(data: &[u8]) -> Vec<u8> {
        let mut decoder = flate2::write::GzDecoder::new(Vec::<u8>::new());
        decoder.write_all(data).unwrap();
        decoder.finish().unwrap()
    }

    pub fn gzip(data: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::<u8>::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, rkyv::Archive, rkyv::Deserialize)]
pub struct WorldInfo {
    pub seed: u32,
}

#[derive(Error, Debug)]
pub enum ChunkLoadError {
    #[error("{0:?}")]
    ParseError(#[from] ParseIntError),
    #[error("{0:?}")]
    IoError(#[from] io::Error),
    #[error("invalid file name")]
    InvalidFileName(String),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, rkyv::Archive, rkyv::Deserialize)]
pub struct PlayerData {
    pub camera: Camera,
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub break_progress: Duration,
    pub break_position: Option<Vector3<i32>>,
    pub time_since_ground: Duration,
    pub movement_mode: i32,
    pub break_cooldown: Duration,

    pub inventory: InventoryData,
    pub attributes: FxHashMap<EntityAttribute, f32>,
    pub health: f32,
}

impl PlayerData {
    pub fn from_real(player: &Player) -> Self {
        Self {
            camera: player.camera.clone(),
            break_cooldown: player.break_cooldown,
            break_position: player.break_position,
            break_progress: player.break_progress,
            health: player.health,
            movement_mode: player.movement_mode,
            position: player.position,
            velocity: player.velocity,
            time_since_ground: player.time_since_ground,
            attributes: player.attributes.clone(),
            inventory: InventoryData {
                selected: player.inventory.selected,
                items: player.inventory.items.iter().map(|it| ItemStackData { id: it.stack.item.into(), count: it.stack.count }).collect()
            }
        }
    }

    pub fn to_real(&self, registries: &Registries) -> Player {
        Player {
            camera: self.camera.clone(),
            break_position: self.break_position,
            break_progress: self.break_progress,
            break_cooldown: self.break_cooldown,
            health: self.health,
            movement_mode: self.movement_mode,
            position: self.position,
            velocity: self.velocity,
            time_since_ground: self.time_since_ground,
            attributes: self.attributes.clone(),
            inventory: Inventory {
                selected: self.inventory.selected,
                items: self.inventory.items.iter().map(|it| InventoryItemStack::new(ItemStack { item: registries.item_registry.get_item(&it.id).id, count: it.count }, registries)).collect()
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, rkyv::Archive, rkyv::Deserialize)]
pub struct InventoryData {
    pub items: Vec<ItemStackData>,
    pub selected: usize,
}