struct VertexInput {
    @location(0) position: vec2<f32>,  // Quad vertex position
}

struct InstanceInput {
    @location(1) center: vec2<f32>,    // Circle center (NDC)
    @location(2) radius: f32,          // Circle radius (NDC)
    @location(3) color: vec3<f32>,     // Circle color (RGB)
}

// Data passed from vertex shader to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // Required output
    @location(0) local_pos: vec2<f32>,            // Position relative to circle center
    @location(1) color: vec3<f32>,                // Circle color
    @location(2) radius: f32,                     // Circle radius
}

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Scale quad by radius, then translate to circle center
    let world_pos = vertex.position * instance.radius + instance.center;
    
    // Output final NDC position (GPU needs vec4 with z=0, w=1 for 2D)
    out.clip_position = vec4<f32>(world_pos, 0.0, 1.0);
    
    // Pass local position (for distance check in fragment shader)
    out.local_pos = vertex.position * instance.radius;
    
    // Pass through color and radius
    out.color = instance.color;
    out.radius = instance.radius;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.local_pos);

    // How wide the edge should be (in local space)
    let edge_width = fwidth(dist);

    // Smooth alpha transition at the circle boundary
    let alpha = 1.0 - smoothstep(
        in.radius - edge_width,
        in.radius + edge_width,
        dist,
    );

    return vec4<f32>(in.color, alpha);
}
