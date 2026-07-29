use winit::event_loop::EventLoop;
use runner::common_main;

mod game;
mod instance;
mod runner;
mod chunk;
mod blocks;
mod util;
mod player;
mod block_models;

include!(concat!(env!("OUT_DIR"), "/resources.rs"));

#[tokio::main]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    common_main(event_loop).await;
}