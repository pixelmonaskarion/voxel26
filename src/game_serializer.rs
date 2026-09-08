use std::{fs::{self, create_dir_all, read, write}, io::{self, Write}, num::ParseIntError, path::PathBuf};

use bespoke_engine::resource_compiler::dir_contents;
use directories::ProjectDirs;
use flate2::Compression;
use glam::ivec3;
use itertools::Itertools;
use rkyv::rancor;
use thiserror::Error;

use crate::{chunk::{ArchivedChunkData, Chunk, ChunkData}, game::Game, player::{ArchivedPlayer, Player}};

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
        let world_info_file = data_dir.join("world_info.rkyv");
        if world_info_file.exists() {
            if let Ok(Ok(Ok(world_info))) = read(world_info_file).map(|file| rkyv::access::<ArchivedWorldInfo, rancor::Error>(&Self::ungzip(&file)).map(|archive| rkyv::deserialize::<WorldInfo, rancor::Error>(archive))) {
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
            if !chunk.data.generated_blocks {
                continue;
            }
            let chunk_file = world_dir.join(format!("x{}y{}z{}.rkyv", pos[0], pos[1], pos[2]));
            write(chunk_file, Self::gzip(&rkyv::to_bytes::<rancor::Error>(&chunk.data).unwrap())).unwrap();
        }
        //player
        let player_file = data_dir.join("player.rkyv");
        write(player_file, Self::gzip(&rkyv::to_bytes::<rancor::Error>(&game.player).unwrap())).unwrap();
        //world_info
        let player_file = data_dir.join("world_info.rkyv");
        write(player_file, Self::gzip(&rkyv::to_bytes::<rancor::Error>(&game.world_info).unwrap())).unwrap();
    }

    pub fn load_world(&mut self, game: &mut Game) {
        let data_dir = self.project_dirs.data_dir().to_path_buf();
        let world_dir = data_dir.join("world");
        let player_file = data_dir.join("player.rkyv");
        if world_dir.exists() && player_file.exists() {
            let player: Player =  rkyv::deserialize::<Player, rancor::Error>(rkyv::access::<ArchivedPlayer, rancor::Error>(&Self::ungzip(&fs::read(player_file).unwrap())).unwrap()).unwrap();
            game.player = player;
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
        let pos = ivec3(x, y, z);
        let chunk_bytes = Self::ungzip(&read(chunk_file).unwrap());
        let chunk_data = rkyv::deserialize::<ChunkData, rancor::Error>(rkyv::access::<ArchivedChunkData, rancor::Error>(&chunk_bytes).unwrap()).unwrap();
        game.chunk_manager.replace_chunk(pos, Chunk {
            creating_model: false,
            data: chunk_data,
            lod: 0,
            model: None,
            needed_chunk_updates: vec![],
        });
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

#[derive(serde::Serialize, serde::Deserialize, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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