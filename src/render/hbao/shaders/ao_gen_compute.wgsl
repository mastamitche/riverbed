#import bevy_render::view::View

struct AOParams {
    radius: f32,
    bias: f32,
    strength: f32,
    num_directions: u32,
    num_steps: u32,
    max_radius_pixels: f32,
    falloff_scale: f32,
    denoise_blur: f32,
}

@group(0) @binding(0) var linear_depth_texture: texture_2d<f32>;; // unfilterable
@group(0) @binding(1) var normal_texture: texture_2d<f32>;; // unfilterable
@group(0) @binding(2) var random_rotation_texture: texture_2d<f32>;; // filterable
@group(0) @binding(3) var output_ao: texture_storage_2d<r32float, write>;
@group(0) @binding(4) var<uniform> ao_params: AOParams;
@group(1) @binding(0) var non_filtering_sampler: sampler;
@group(1) @binding(1) var filtering_clamp_sampler: sampler;
@group(1) @binding(2) var filtering_wrap_sampler: sampler;
@group(1) @binding(3) var<uniform> view: View;

fn reconstruct_position(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    // Convert UV to NDC
    let ndc = vec2<f32>(uv * 2.0 - 1.0);
    
    // Reconstruct view-space position
    let clip_pos = vec4<f32>(ndc.x, ndc.y, depth, 1.0);
    
    // Use world_from_clip instead of projection_inverse
    let world_pos = view.world_from_clip * clip_pos;
    
    // If you need view space instead of world space, you can do:
    // let view_pos = view.view_from_world * world_pos;
    // return view_pos.xyz / view_pos.w;
    
    // But if world space is sufficient:
    return world_pos.xyz / world_pos.w;
}
fn sample_random_vector(coords: vec2<i32>) -> vec2<f32> {
    let dims = textureDimensions(random_rotation_texture);
    let dims_i32 = vec2<i32>(dims);
    
    // Wrap the coordinates
    let wrapped_coords = ((coords % dims_i32) + dims_i32) % dims_i32;
    
    // Calculate UVs for textureGather
    let gather_uv = (vec2<f32>(wrapped_coords) + 0.5) / vec2<f32>(dims);
    
    // Use textureGather to get 4 texels at once (returns the red component from 4 adjacent texels)
    let random_r = textureGather(0, random_rotation_texture, filtering_wrap_sampler, gather_uv);
    let random_g = textureGather(1, random_rotation_texture, filtering_wrap_sampler, gather_uv);
    
    // Use the first sample (you could average them if desired)
    let random = vec2<f32>(random_r.w, random_g.w);
    
    return random * 2.0 - 1.0;
}

fn calculate_ao(origin_pos: vec3<f32>, normal: vec3<f32>, random_vec: vec2<f32>, uv: vec2<f32>, texel_size: vec2<f32>) -> f32 {
    // Calculate tangent and bitangent for sampling hemisphere
    var tangent = normalize(vec3<f32>(random_vec.x, random_vec.y, 0.0));
    if (abs(dot(tangent, normal)) > 0.99) {
        tangent = normalize(vec3<f32>(0.0, 1.0, 0.0));
    }
    let bitangent = normalize(cross(normal, tangent));
    tangent = cross(bitangent, normal);
    
    // Matrix for rotating samples
    let tbn = mat3x3<f32>(tangent, bitangent, normal);
    
    var total_ao = 0.0;
    let step_angle = 2.0 * 3.14159265359 / f32(ao_params.num_directions);
    
    // Sample in multiple directions
    for (var i = 0u; i < ao_params.num_directions; i = i + 1u) {
        let angle = f32(i) * step_angle;
        let direction = vec2<f32>(cos(angle), sin(angle));
        
        // Rotate with random vector to reduce banding
        let rotated_dir = vec2<f32>(
            direction.x * random_vec.x - direction.y * random_vec.y,
            direction.x * random_vec.y + direction.y * random_vec.x
        );
        
        // Calculate horizon occlusion
        let max_angle = -1.0;
        let step_size = ao_params.radius / f32(ao_params.num_steps);
        
        // Sample along the direction
        for (var j = 1u; j <= ao_params.num_steps; j = j + 1u) {
            let sample_dist = step_size * f32(j);
            
            // Calculate sample point in screen space
            let sample_uv = uv + rotated_dir * sample_dist * texel_size * ao_params.max_radius_pixels;
            
            // Skip samples outside the screen
            if (sample_uv.x <= 0.0 || sample_uv.x >= 1.0 || sample_uv.y <= 0.0 || sample_uv.y >= 1.0) {
                continue;
            }
            
            // textureGather returns 4 samples in counterclockwise order starting from the bottom-left
            let depths = textureGather(0, linear_depth_texture, non_filtering_sampler, sample_uv);
            // Use bilinear interpolation manually if needed, or just pick one sample
            // For simplicity, let's use the bottom-left sample (depths.w)
            let sample_depth = depths.w;
            let sample_pos = reconstruct_position(sample_uv, sample_depth);

            // Calculate vector from origin to sample
            let v = sample_pos - origin_pos;
            let dist = length(v);
            
            // Skip if sample is too far
            if (dist > ao_params.radius) {
                continue;
            }
            
            // Calculate angle between normal and sample
            let normalized_v = v / dist;
            let angle = asin(clamp(normalized_v.z, -1.0, 1.0));
            
            // Apply falloff based on distance
            let falloff = 1.0 - pow(dist / ao_params.radius, ao_params.falloff_scale);
            
            // Accumulate occlusion with bias
            if (angle > max_angle) {
                let horizon_angle = angle;
                let occlusion = clamp(sin(horizon_angle) - sin(max_angle + ao_params.bias), 0.0, 1.0);
                total_ao += occlusion * falloff;
            }
        }
    }
    
    // Normalize and apply strength
    total_ao = 1.0 - (total_ao / f32(ao_params.num_directions) * ao_params.strength);
    return clamp(total_ao, 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn ao_gen_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(linear_depth_texture);
    let coords = vec2<i32>(global_id.xy);
    
    let dims_i32 = vec2<i32>(dims);
    if (coords.x >= dims_i32.x || coords.y >= dims_i32.y) {
        return;
    }

    let uv = (vec2<f32>(coords) + 0.5) / vec2<f32>(dims);
    let texel_size = 1.0 / vec2<f32>(dims);
    
    // Get depth and normal
    let depth = textureLoad(linear_depth_texture, coords, 0).r;
    let normal = textureLoad(normal_texture, coords, 0).xyz * 2.0 - 1.0;  // Unpack from [0,1] to [-1,1]
    
    // Skip if depth is too far (e.g., sky)
    if (depth >= 0.9999) {
        textureStore(output_ao, coords, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }
    
    // Reconstruct view position
    let position = reconstruct_position(uv, depth);
    
    // Get random rotation vector for this pixel
    let random_vec = sample_random_vector(coords);
    
    // Calculate AO
    let ao_value = calculate_ao(position, normal, random_vec, uv, texel_size);
    
    // Store AO value
    textureStore(output_ao, coords, vec4<f32>(ao_value, 0.0, 0.0, 0.0));
}