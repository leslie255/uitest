@group(0) @binding(0) var<uniform> projection: mat4x4<f32>;

const vertices = array<vec2<f32>, 6>(
    vec2<f32>(0., 0.),
    vec2<f32>(1., 0.),
    vec2<f32>(1., 1.),
    vec2<f32>(0., 0.),
    vec2<f32>(1., 1.),
    vec2<f32>(0., 1.),
);

struct InstanceInput {
    @location(0) model_view_col_0: vec3<f32>,
    @location(1) model_view_col_1: vec3<f32>,
    @location(2) model_view_col_2: vec3<f32>,
    @location(3) fill_color: vec4<f32>,
    @location(4) line_color: vec4<f32>,
    @location(5) line_width: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fill_color: vec4<f32>,
    @location(2) line_color: vec4<f32>,
    @location(3) line_width: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32, instance: InstanceInput) -> VertexOutput {
    var result: VertexOutput;
    let position = vertices[index];
    result.uv = position;
    let model_view = mat3x3<f32>(
            instance.model_view_col_0,
            instance.model_view_col_1,
            instance.model_view_col_2,
        );
    let position_transformed = model_view * vec3<f32>(position.xy, 1.);
    result.position = projection * vec4<f32>(position_transformed.xy, 0.0, 1.0);
    result.fill_color = instance.fill_color;
    result.line_color = instance.line_color;
    result.line_width = instance.line_width;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let distance = min(vertex.uv, vec2<f32>(1., 1.) - vertex.uv);
    return select(
            vertex.fill_color,
            vertex.line_color,
            distance.x < vertex.line_width.x || distance.y < vertex.line_width.y,
        );
}
