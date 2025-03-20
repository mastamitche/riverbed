#import bevy_render::view::View

struct BlurParams {
    blur_radius: u32,
    sharpness: f32,
    normal_sensitivity: f32,
    depth_sensitivity: f32,
}

@group(0) @binding(0) var ao_input: texture_2d<f32>;
@group(0) @binding(1) var linear_depth_texture: texture_2d<f32>;
@group(0) @binding(2) var normal_texture: texture_2d<f32>;
@group(0) @binding(3) var output_final: texture_storage_2d<r32uint, write>;
@group(0) @binding(4) var<uniform> blur_params: BlurParams;
@group(1) @binding(0) var non_filtering_sampler: sampler;
@group(1) @binding(1) var filtering_clamp_sampler: sampler;
@group(1) @binding(2) var filtering_wrap_sampler: sampler;
@group(1) @binding(3) var<uniform> view: View;

fn calculate_weight(
    center_depth: f32, 
    sample_depth: f32, 
    center_normal: vec3<f32>, 
    sample_normal: vec3<f32>
) -> f32 {
    let depth_diff = abs(center_depth - sample_depth);
    let depth_weight = exp(-depth_diff * blur_params.depth_sensitivity / center_depth);
    let normal_dot = max(0.0, dot(center_normal, sample_normal));
    let normal_weight = pow(normal_dot, blur_params.normal_sensitivity);
    return depth_weight * normal_weight;
}

@compute @workgroup_size(8, 8, 1)
fn blur_combined(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(ao_input);
    let coords = vec2<i32>(global_id.xy);
    
    // Convert coords to unsigned before comparison
    if (u32(coords.x) >= dims.x || u32(coords.y) >= dims.y) {
        return;
    }
    
    let center_ao = textureLoad(ao_input, coords, 0).r;
    let center_depth = textureLoad(linear_depth_texture, coords, 0).r;
    let center_normal = textureLoad(normal_texture, coords, 0).xyz * 2.0 - 1.0;
    
    if (center_depth >= 0.9999) {
        let ao_uint = u32(center_ao * 255.0); // Scale to 0-255 range
        textureStore(output_final, coords, vec4<u32>(ao_uint, 0u, 0u, 0u));
        return;
    }
    
    var sum = center_ao;
    var total_weight = 1.0;
    let radius = i32(blur_params.blur_radius);
    
    // Combined horizontal and vertical blur
    for (var y = -radius; y <= radius; y = y + 1) {
        for (var x = -radius; x <= radius; x = x + 1) {
            // Skip center pixel
            if (x == 0 && y == 0) {
                continue;
            }
                        
            let dims_i32 = vec2<i32>(dims);
            let sample_pos = coords + vec2<i32>(x, y);
                        
            // Skip samples outside the texture
            if (sample_pos.x < 0 || sample_pos.x >= dims_i32.x || 
                sample_pos.y < 0 || sample_pos.y >= dims_i32.y) {
                continue;
            }
            
            let sample_ao = textureLoad(ao_input, sample_pos, 0).r;
            let sample_depth = textureLoad(linear_depth_texture, sample_pos, 0).r;
            let sample_normal = textureLoad(normal_texture, sample_pos, 0).xyz * 2.0 - 1.0;
            
            let weight = calculate_weight(center_depth, sample_depth, center_normal, sample_normal);
            
            // Apply bilateral weight
            let kernel_falloff = exp(-f32(x * x + y * y) / f32(blur_params.sharpness));
            let final_weight = weight * kernel_falloff;
            
            sum += sample_ao * final_weight;
            total_weight += final_weight;
        }
    }
    
    let blurred_ao = sum / total_weight;
    let ao_uint = u32(blurred_ao * 255.0); // Scale to 0-255 range
    textureStore(output_final, coords, vec4<u32>(ao_uint, 0, 0, 0));
}