
struct VertexInput {
      @location(0) position: vec3<f32>,
      @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  output.position = vec4<f32>(input.position, 1.0);
  output.tex_coords = input.tex_coords;
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(input.tex_coords, 0.0, 1.0);
}
