@group(0) @binding(0) var<uniform> model_view: mat4x4<f32>;
@group(0) @binding(1) var<uniform> projection: mat4x4<f32>;
@group(0) @binding(2) var<uniform> fill_color: vec4<f32>;
@group(0) @binding(3) var<uniform> line_color: vec4<f32>;
@group(0) @binding(4) var<uniform> line_width: vec4<f32>;

const vertices = array<vec2<f32>, 6>(
    vec2<f32>(0., 0.),
    vec2<f32>(1., 0.),
    vec2<f32>(1., 1.),
    vec2<f32>(0., 0.),
    vec2<f32>(1., 1.),
    vec2<f32>(0., 1.),
);

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var result: VertexOutput;
    let position = vertices[index];
    result.uv = position;
    result.position = projection * model_view * vec4<f32>(position.xy, 0.0, 1.0);
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let distance = vec4<f32>(vertex.uv, vec2<f32>(1.) - vertex.uv).xzyw;
    return select(
            fill_color,
            line_color,
            any(distance < line_width),
        );
}
