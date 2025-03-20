#import bevy_render::view::View

struct AOApplicationParams {
    strength: f32,
    power: f32,
    distance_falloff_min: f32,
    distance_falloff_max: f32,
    use_distance_falloff: u32,
    multiply_mode: u32,  // 0 = multiply, 1 = overlay
    color_bleed_intensity: f32,
    ao_color: vec3<f32>,
}

@group(0) @binding(0) var<uniform> ao_params: AOApplicationParams;
@group(0) @binding(1) var ao_texture: texture_2d<u32>; // filterable
@group(0) @binding(2) var linear_depth_texture: texture_2d<f32>;// unfiterable
@group(1) @binding(0) var non_filtering_sampler: sampler;
@group(1) @binding(1) var filtering_clamp_sampler: sampler;
@group(1) @binding(2) var filtering_wrap_sampler: sampler;
@group(1) @binding(3) var<uniform> view: View;

// Overlay blend mode function
fn overlay_blend(base: f32, blend: f32) -> f32 {
    var result: f32;
    if (base < 0.5) {
        result = 2.0 * base * blend;
    } else {
        result = 1.0 - 2.0 * (1.0 - base) * (1.0 - blend);
    }
    return result;
}

fn overlay_blend_rgb(base: vec3<f32>, blend: f32) -> vec3<f32> {
    return vec3<f32>(
        overlay_blend(base.r, blend),
        overlay_blend(base.g, blend),
        overlay_blend(base.b, blend)
    );
}

fn apply_distance_falloff(ao_value: u32, depth: f32) -> f32 {
    if (ao_params.use_distance_falloff == 0u) {
        return f32(ao_value) / 255.0;
    }
    
    // Convert linear depth to view space distance
    let distance = depth;
    
    // Calculate falloff factor
    let falloff_start = ao_params.distance_falloff_min;
    let falloff_end = ao_params.distance_falloff_max;
    
    let falloff_factor = smoothstep(falloff_start, falloff_end, distance);
    
    // Blend AO with full ambient (1.0) based on distance
    return mix(f32(ao_value) / 255.0, 1.0, falloff_factor);
}
@fragment
fn apply_ao_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(i32(position.x), i32(position.y));
    
    // Sample AO and depth
    let ao = textureLoad(ao_texture, coords, 0).r;
    let depth = textureLoad(linear_depth_texture, coords, 0).r;
    
    // Process AO
    let ao_with_falloff = apply_distance_falloff(ao, depth);
    let ao_powered = pow(ao_with_falloff, ao_params.power);
    let final_ao = mix(1.0, ao_powered, ao_params.strength);
    
    // Output the AO directly as a grayscale value
    return vec4<f32>(final_ao, final_ao, final_ao, 1.0);
}
