// Vertex shader
@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(vertex_index & 1u) * 2.0 - 1.0;
    let y = f32((vertex_index >> 1u) & 1u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// Uniform bindings
struct ViewUniform {
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    projection: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    world_position: vec3<f32>,
    width: f32,
    height: f32,
    near: f32,
    far: f32,
    time: f32,
    delta_time: f32,
}

struct NormalAoSettings {
    intensity: f32,
    radius: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: ViewUniform;
@group(0) @binding(1) var color_texture: texture_2d<f32>;
@group(0) @binding(2) var normal_texture: texture_2d<f32>;
@group(0) @binding(3) var texture_sampler: sampler;
@group(0) @binding(4) var<uniform> settings: NormalAoSettings;

// Fragment shader
@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(position.xy / vec2<f32>(view.width, view.height));
    let color = textureSample(color_texture, texture_sampler, uv);
    let normal = textureSample(normal_texture, texture_sampler, uv).xyz * 2.0 - 1.0;
    
    // Simple ambient occlusion effect
    var ao: f32 = 1.0;
    let radius = settings.radius;
    let intensity = settings.intensity;
    
    // Sample points in a hemisphere oriented along the normal
    let samples = 4;
    for (var i = 0; i < samples; i++) {
        let angle = f32(i) * 6.28318 / f32(samples);
        let offset = vec2<f32>(cos(angle), sin(angle)) * radius / vec2<f32>(view.width, view.height);
        
        // Sample depth at offset position
        let offset_color = textureSample(color_texture, texture_sampler, uv + offset);
        let offset_normal = textureSample(normal_texture, texture_sampler, uv + offset).xyz * 2.0 - 1.0;
        
        // Calculate occlusion based on normal difference
        let normal_diff = max(0.0, dot(normal, offset_normal));
        ao -= (1.0 - normal_diff) * (intensity / f32(samples));
    }
    
    ao = clamp(ao, 0.0, 1.0);
    
    // Apply AO to the color
    return vec4<f32>(color.rgb * ao, color.a);
}
