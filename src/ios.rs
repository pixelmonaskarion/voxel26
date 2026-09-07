#![no_main]
use futures::executor::block_on;
use tokio::runtime::Runtime;
use winit::event_loop::EventLoop;

mod game;
mod instance;
mod runner;
mod chunk;
mod blocks;
mod util;
mod player;
mod block_models;
mod features;
mod cube_outline;
mod inventory;
mod ui;
mod particles;
mod entity;
mod ssao;
mod items;
mod registries;
mod crafting;
mod game_serializer;
mod tags;
mod const_block_models;

include!(concat!(env!("OUT_DIR"), "/resources.rs"));

#[unsafe(no_mangle)]
// #[cfg(target_os = "ios")] 
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    actual_main();
    return 1;
}

#[unsafe(no_mangle)]
// #[cfg(target_os = "ios")] 
pub extern "C" fn actual_main() {
    println!("starting");
    let rt = Runtime::new().unwrap();
    println!("blocking");
    rt.block_on(async {
        env_logger::init();
        let event_loop = EventLoop::new().unwrap();
        runner::common_main(event_loop).await;
    });
    println!("finished main");
}