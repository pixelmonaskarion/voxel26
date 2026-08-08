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
    position: vec4f,
    color: vec4f,
    velocity: vec4f,
    lifetime: u32,
    particle_type: u32,
    padding: vec2f,
}