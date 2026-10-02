struct ScreenInfo {
    screen_size: vec2f,
    time: f32,
    camera: Camera,
}

struct VoxelVertex {
    @location(0) position: vec4f,
    @location(1) color: vec4f,
    @location(2) normal: vec4f,
}

struct ParticleInstance {
    @location(5) position: vec4<f32>,
    @location(6) color: vec4<f32>,
}

struct Particle {
    lifetime: f32,
    particle_type: u32,
    padding: vec2f,
    position: vec4f,
    color: vec4f,
    velocity: vec4f,
}

struct DeferredFragmentOutput {
  @location(0) color: vec4f,
  @location(1) normal: vec4f,
  @location(2) worldspace: vec4f,
}

fn mix_colors(back: vec4f, front: vec4f) -> vec4f{
    let pre_back = vec4f(back.rgb * back.a, back.a);
    let pre_front = vec4f(front.rgb * front.a, front.a);
    let final_rgb = back.rgb + (front.rgb * (1 - back.a));       
    let final_a = back.a + (front.a * (1.0 - back.a));
    return vec4f(final_rgb, final_a);
}