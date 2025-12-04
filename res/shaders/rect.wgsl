@group(0) @binding(0) var<uniform> model_view: mat4x4<f32>;
@group(0) @binding(1) var<uniform> projection: mat4x4<f32>;
@group(0) @binding(2) var<uniform> fill_color: vec4<f32>;

struct VertexInput {
    // @builtin(vertex_index) index: u32,
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var result: VertexOutput;
    result.uv = input.uv;
    result.position = projection * model_view * vec4<f32>(input.position.xy, 0.0, 1.0);
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4<f32>(0.214, 0.214, 0.214, 1.0);
    return fill_color;
}
