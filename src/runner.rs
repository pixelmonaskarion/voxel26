use bespoke_engine::{surface_context::SurfaceCtx, window::Surface};
use winit::{dpi::PhysicalPosition, event_loop::EventLoop};

use crate::game::Game;

#[allow(dead_code)]
pub async fn common_main(event_loop: EventLoop<()>) {
    let ready = &|surface_context: &dyn SurfaceCtx| {
        let _ = surface_context.window().set_cursor_visible(false);
        let _ = surface_context.window().set_cursor_position(PhysicalPosition::new(surface_context.config().width/2, surface_context.config().height/2));
        Game::new(surface_context)
    };
    let mut surface = Surface::new(ready).await;
    event_loop.run_app(&mut surface).unwrap();
    println!("exiting");
}