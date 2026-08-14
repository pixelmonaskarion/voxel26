use bespoke_engine::{binding::{Descriptor, UniformBinding}, culling::AABB, model::Model, resource_loader::load_resource_string, shader::{Shader, ShaderConfig, ShaderType, parse_shader}};
use cgmath::{InnerSpace, Matrix, Matrix3, Matrix4, SquareMatrix, Vector3, Vector4};
use wgpu::{Device, FrontFace, PipelineCompilationOptions, TextureFormat};

use crate::{game::{ScreenInfo, Vertex}, instance::Instance};

pub fn cube_outline_shader<'a>(device: &Device, formats: Vec<TextureFormat>, screen_info_binding: &UniformBinding<ScreenInfo>) -> Shader<'a> {
    let resource_path = "res/shaders/cube_outline.wgsl";
    let shader_types: Vec<&ShaderType> = vec![&screen_info_binding.shader_type];
    let bindings = vec![&screen_info_binding.layout];
    let vertex_buffers = vec![Some(Vertex::desc()), Some(Instance::desc())];
    let config = ShaderConfig {
        face_cull: None,
        // depth_compare: wgpu::CompareFunction::Always,
        ..Default::default()
    };

    let source = &load_resource_string(resource_path);
    let shader_types_owned = shader_types.clone().into_iter().map(|it| it.clone()).collect();
    let parsed_source = parse_shader(source, &shader_types_owned);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(parsed_source.clone().into()),
    });
    let targets = &formats.iter().map(|format| {
        Some(wgpu::ColorTargetState {
            format: *format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })
    }).collect::<Vec<Option<wgpu::ColorTargetState>>>();
    let fragment = if !config.depth_only {
        Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets,
            compilation_options: PipelineCompilationOptions::default(),
        })
    } else {
        None
    };
    let layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            immediate_size: 0,
            bind_group_layouts: &bindings.into_iter().map(|it| Some(it)).collect::<Vec<_>>(),
        });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &vertex_buffers,
            compilation_options: PipelineCompilationOptions::default(),
        },
        depth_stencil: config.depth_stencil(),
        fragment,
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: config.face_cull.unwrap_or(FrontFace::Ccw),
            cull_mode: config.face_cull.map(|_| wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
            // or Features::POLYGON_MODE_POINT
            polygon_mode: config.line_mode,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        // If the pipeline will be used with a multiview render pass, this
        // indicates how many array layers the attachments will have.
        multiview_mask: None,
        cache: None,
    });
    Shader {
        shader,
        layout,
        pipeline,
        resource_path: resource_path.into(),
        config,
        vertex_buffers,
        shader_types: shader_types_owned,
        formats,
    }
}

pub fn cube_outline_model(device: &Device, view_proj: Matrix4<f32>, position: Vector3<f32>) -> Model {
    let (vertices, indices) = generate_thick_wireframe_cube(view_proj, [1.0; 4]);
    Model::new_instances(vertices, &indices, vec![Instance { position, ..Default::default() }], AABB::zero(), device)
}

/// Generates a thick wireframe cube spanning (0,0,0) to (1,1,1): each of
/// the 12 edges is its own box, built from quads (two triangles per face).
/// Positions are transformed by `view_proj`; `color` is applied to every vertex.
pub fn generate_thick_wireframe_cube(
    view_proj: Matrix4<f32>,
    color: [f32; 4],
) -> (Vec<Vertex>, Vec<u32>) {
    generate_thick_wireframe_cube_with_params(view_proj, color, 1.0, 0.02)
}

/// Same as `generate_thick_wireframe_cube` but with explicit control over
/// the cube's edge length (`size`) and the wire `thickness`. The cube spans
/// (0,0,0) to (size,size,size).
pub fn generate_thick_wireframe_cube_with_params(
    view_proj: Matrix4<f32>,
    color: [f32; 4],
    size: f32,
    thickness: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let t = thickness * 0.5;
    let lo = 0.0f32;
    let hi = size;

    // Inverse-transpose of the linear part of view_proj, for correct normal
    // transformation under non-uniform scale/skew. Swap `view_proj` for a
    // dedicated model matrix here if you need normals usable for lighting.
    let normal_mat3 = {
        let m3 = Matrix3::from_cols(
            view_proj.x.truncate(),
            view_proj.y.truncate(),
            view_proj.z.truncate(),
        );
        m3.invert().map(|m| m.transpose()).unwrap_or(m3)
    };

    // Edges running along X: vary y and z between the two cube extremes.
    for &y in &[lo, hi] {
        for &z in &[lo, hi] {
            add_box(
                &mut vertices,
                &mut indices,
                Vector3::new(lo - t, y - t, z - t),
                Vector3::new(hi + t, y + t, z + t),
                color,
                &normal_mat3,
            );
        }
    }

    // Edges running along Y.
    for &x in &[lo, hi] {
        for &z in &[lo, hi] {
            add_box(
                &mut vertices,
                &mut indices,
                Vector3::new(x - t, lo - t, z - t),
                Vector3::new(x + t, hi + t, z + t),
                color,
                &normal_mat3,
            );
        }
    }

    // Edges running along Z.
    for &x in &[lo, hi] {
        for &y in &[lo, hi] {
            add_box(
                &mut vertices,
                &mut indices,
                Vector3::new(x - t, y - t, lo - t),
                Vector3::new(x + t, y + t, hi + t),
                color,
                &normal_mat3,
            );
        }
    }

    (vertices, indices)
}

/// Appends a single axis-aligned box (6 quad faces, flat-shaded normals)
/// to the mesh, transforming each vertex position by `view_proj`.
fn add_box(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    min: Vector3<f32>,
    max: Vector3<f32>,
    color: [f32; 4],
    normal_mat3: &Matrix3<f32>,
) {
    // (face normal, 4 corners in CCW winding when viewed from outside)
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        // +X
        (
            [1.0, 0.0, 0.0],
            [
                [max.x, min.y, min.z],
                [max.x, max.y, min.z],
                [max.x, max.y, max.z],
                [max.x, min.y, max.z],
            ],
        ),
        // -X
        (
            [-1.0, 0.0, 0.0],
            [
                [min.x, min.y, max.z],
                [min.x, max.y, max.z],
                [min.x, max.y, min.z],
                [min.x, min.y, min.z],
            ],
        ),
        // +Y
        (
            [0.0, 1.0, 0.0],
            [
                [min.x, max.y, min.z],
                [min.x, max.y, max.z],
                [max.x, max.y, max.z],
                [max.x, max.y, min.z],
            ],
        ),
        // -Y
        (
            [0.0, -1.0, 0.0],
            [
                [min.x, min.y, max.z],
                [min.x, min.y, min.z],
                [max.x, min.y, min.z],
                [max.x, min.y, max.z],
            ],
        ),
        // +Z
        (
            [0.0, 0.0, 1.0],
            [
                [min.x, min.y, max.z],
                [max.x, min.y, max.z],
                [max.x, max.y, max.z],
                [min.x, max.y, max.z],
            ],
        ),
        // -Z
        (
            [0.0, 0.0, -1.0],
            [
                [max.x, min.y, min.z],
                [min.x, min.y, min.z],
                [min.x, max.y, min.z],
                [max.x, max.y, min.z],
            ],
        ),
    ];

    for (normal, quad) in faces.iter() {
        let base_index = vertices.len() as u32;

        let n = normal_mat3 * Vector3::new(normal[0], normal[1], normal[2]);
        let n = if n.magnitude2() > 0.0 { n.normalize() } else { n };

        for p in quad.iter() {
            let world_pos = Vector4::new(p[0], p[1], p[2], 1.0);

            vertices.push(Vertex {
                position: [world_pos.x, world_pos.y, world_pos.z, world_pos.w],
                color,
                normal: [n.x, n.y, n.z, 0.0],
            });
        }

        // Two triangles per quad, CCW winding.
        indices.push(base_index);
        indices.push(base_index + 1);
        indices.push(base_index + 2);
        indices.push(base_index);
        indices.push(base_index + 2);
        indices.push(base_index + 3);
    }
}