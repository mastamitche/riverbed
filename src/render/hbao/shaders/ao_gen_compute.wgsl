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
fn calculate_ao_voxel_balanced(position: vec3<f32>, normal: vec3<f32>, random_vec: vec2<f32>, uv: vec2<f32>, texel_size: vec2<f32>, depth: f32) -> f32 {
    var tangent = normalize(vec3<f32>(random_vec.x, random_vec.y, 0.0));
    if (abs(dot(tangent, normal)) > 0.99) {
        tangent = normalize(vec3<f32>(0.0, 1.0, 0.0));
    }
    let bitangent = normalize(cross(normal, tangent));
    tangent = cross(bitangent, normal);
    
    var total_ao = 0.0;
    let num_directions = max(ao_params.num_directions, 16u);
    let step_angle = 2.0 * 3.14159265359 / f32(num_directions);
    
    // Ensure we sample both horizontal and vertical directions evenly
    for (var i = 0u; i < num_directions; i = i + 1u) {
        let angle = f32(i) * step_angle;
        let direction = vec2<f32>(cos(angle), sin(angle));
        
        // Rotate with random vector to reduce banding
        let rotated_dir = vec2<f32>(
            direction.x * random_vec.x - direction.y * random_vec.y,
            direction.x * random_vec.y + direction.y * random_vec.x
        );
        
        var occlusion_sum = 0.0;
        var prev_depth = depth;
        
        // Use a smaller radius but more steps for fine detail
        let sample_radius = ao_params.radius * 0.2;
        let num_steps = min(ao_params.num_steps * 3u, 24u);
        let step_size = sample_radius / f32(num_steps);
        
        // Explicitly boost vertical sensitivity
        let vertical_boost = 1.0 + 2.0 * abs(direction.y);
        
        for (var j = 1u; j <= num_steps; j = j + 1u) {
            let sample_dist = step_size * f32(j);
            
            // Use pixel-based sampling for more precision
            let sample_uv = uv + rotated_dir * sample_dist * texel_size * ao_params.max_radius_pixels;
            
            if (sample_uv.x <= 0.0 || sample_uv.x >= 1.0 || sample_uv.y <= 0.0 || sample_uv.y >= 1.0) {
                continue;
            }
            
            // Get precise sample coordinates
            let dims = textureDimensions(linear_depth_texture);
            let sample_coords = vec2<i32>(sample_uv * vec2<f32>(dims));
            
            // Sample depth directly for better precision
            let sample_depth = textureLoad(linear_depth_texture, sample_coords, 0).r;
            let sample_pos = reconstruct_position(sample_uv, sample_depth);
            
            // Calculate depth difference with vertical boost
            let depth_diff = abs(sample_depth - prev_depth) * 100.0 * vertical_boost;
            
            // Calculate world-space distance
            let world_dist = distance(sample_pos, position);
            
            // Detect edges - more sensitive for vertical directions
            if (depth_diff > 0.001) {
                // The smaller the distance but larger the depth change, the stronger the occlusion
                let edge_strength = min(depth_diff / max(world_dist * 10.0, 0.001), 1.0);
                occlusion_sum += edge_strength;
            }
            
            prev_depth = sample_depth;
        }
        
        // Add weighted occlusion for this direction
        total_ao += min(occlusion_sum, 1.0);
    }
    
    // Normalize and apply strength
    total_ao = 1.0 - (total_ao / f32(num_directions) * ao_params.strength * 2.0);
    
    // Apply moderate contrast - avoid extreme values that might cause only dots
    total_ao = pow(clamp(total_ao, 0.0, 1.0), 0.7);
    
    return total_ao;
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
    let ao_value = calculate_ao_voxel_balanced(position, normal, random_vec, uv, texel_size, depth);
    
    // Store AO value
    textureStore(output_ao, coords, vec4<f32>(ao_value, 0.0, 0.0, 0.0));
}