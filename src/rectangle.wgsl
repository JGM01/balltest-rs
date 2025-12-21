struct VertexInput {
    @location(0) position: vec2<f32>,  // Quad vertex position
}

struct InstanceInput {
    @location(1) center: vec2<f32>,    // Rectangle center (NDC)
    @location(2) length: f32,          // Rectangle width (NDC)
    @location(3) height: f32,          // Rectangle height (NDC)
    @location(4) color: vec3<f32>,     // Rectangle color (RGB)
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Scale quad by dimensions, then translate to center
    let half_size = vec2<f32>(instance.length / 2.0, instance.height / 2.0);
    let world_pos = vertex.position * half_size + instance.center;
    
    out.clip_position = vec4<f32>(world_pos, 0.0, 1.0);
    out.color = instance.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
