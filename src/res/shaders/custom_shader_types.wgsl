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