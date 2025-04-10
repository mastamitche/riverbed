
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types,
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_deferred_functions::deferred_output,
}


struct VoxelMaterial {
    chunk_position: vec3<i32>,
};


@group(2) @binding(100)
var<uniform> material: VoxelMaterial;

@group(2) @binding(101)
var voxel_texture: texture_3d<f32>;

@group(2) @binding(102)
var voxel_sampler: sampler;

fn apply_ambient_occlusion(color: vec4<f32>, ao_factor: f32) -> vec4<f32> {
    return vec4<f32>(color.rgb * ao_factor, color.a);
}

fn calculate_ao(world_pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> f32 {
    // Convert world position to local chunk position
    let local_pos = world_pos - vec3<f32>(material.chunk_position);
    
    // Calculate the exact position within the voxel grid based on UVs
    let voxel_count = 62.0;
    let uv_grid = uv * voxel_count;
    let voxel_coord = floor(uv_grid);
    let fract_pos = fract(uv_grid);
    
    // Determine which face we're on
    let abs_normal = abs(normal);
    
    // Build the voxel position based on the face normal
    var voxel_pos = vec3<f32>(0.0);
    
    if (abs_normal.x > 0.9) {
        // X-aligned face
        voxel_pos = vec3<f32>(
            local_pos.x, // Keep the x-coordinate from world position
            voxel_coord.x,
            voxel_coord.y
        );
    } else if (abs_normal.y > 0.9) {
        // Y-aligned face
        voxel_pos = vec3<f32>(
            voxel_coord.x,
            local_pos.y, // Keep the y-coordinate from world position
            voxel_coord.y
        );
    } else {
        // Z-aligned face
        voxel_pos = vec3<f32>(
            voxel_coord.x,
            voxel_coord.y,
            local_pos.z // Keep the z-coordinate from world position
        );
    }
    
    // Calculate AO for the current position
    // We'll check the 8 surrounding voxels (4 edges, 4 corners) for occlusion
    
    // First, determine the local u,v axes for this face
    var u_axis: vec3<f32>;
    var v_axis: vec3<f32>;
    
    if (abs_normal.x > 0.9) {
        u_axis = vec3<f32>(0.0, 1.0, 0.0);
        v_axis = vec3<f32>(0.0, 0.0, 1.0);
    } else if (abs_normal.y > 0.9) {
        u_axis = vec3<f32>(1.0, 0.0, 0.0);
        v_axis = vec3<f32>(0.0, 0.0, 1.0);
    } else {
        u_axis = vec3<f32>(1.0, 0.0, 0.0);
        v_axis = vec3<f32>(0.0, 1.0, 0.0);
    }
    
    // Calculate the four corners of the current texel
    let corner_offsets = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), // bottom-left
        vec2<f32>(1.0, 0.0), // bottom-right
        vec2<f32>(0.0, 1.0), // top-left
        vec2<f32>(1.0, 1.0)  // top-right
    );
    
    // Sample AO for each corner and interpolate
    var corner_ao = array<f32, 4>(0.0, 0.0, 0.0, 0.0);
    
    for (var i = 0; i < 4; i++) {
        let offset = corner_offsets[i];
        
        // Calculate the exact corner position
        let corner = voxel_pos + u_axis * offset.x + v_axis * offset.y;
        
        // Check voxels around this corner for occlusion
        let voxel_offsets = array<vec3<f32>, 3>(
            normal,                                     // directly in front
            u_axis * (offset.x * 2.0 - 1.0),           // side 1
            v_axis * (offset.y * 2.0 - 1.0)            // side 2
        );
        
        var occlusion = 0.0;
        var valid_samples = 0.0;
        
        for (var j = 0; j < 3; j++) {
            let sample_pos = corner + voxel_offsets[j];
            
            // Convert to texture coordinates
            let tex_coord = sample_pos / vec3<f32>(64.0);
            
            // Skip if outside texture bounds
            if (any(tex_coord < vec3<f32>(0.0)) || any(tex_coord >= vec3<f32>(1.0))) {
                continue;
            }
            
            let voxel_present = textureSample(voxel_texture, voxel_sampler, tex_coord).r;
            occlusion += voxel_present;
            valid_samples += 1.0;
        }
        
        // Also check the diagonal corner
        let diag_pos = corner + normal + u_axis * (offset.x * 2.0 - 1.0) + v_axis * (offset.y * 2.0 - 1.0);
        let diag_tex_coord = diag_pos / vec3<f32>(64.0);
        
        if (!any(diag_tex_coord < vec3<f32>(0.0)) && !any(diag_tex_coord >= vec3<f32>(1.0))) {
            let diag_present = textureSample(voxel_texture, voxel_sampler, diag_tex_coord).r;
            occlusion += diag_present;
            valid_samples += 1.0;
        }
        
        // Calculate AO for this corner
        if (valid_samples > 0.0) {
            let occlusion_factor = occlusion / valid_samples;
            corner_ao[i] = 1.0 - occlusion_factor * 0.8; // Adjust strength
        } else {
            corner_ao[i] = 1.0; // No occlusion if no valid samples
        }
    }
    
    // Bilinear interpolation between the four corners based on fractional position
    let ao_bottom = mix(corner_ao[0], corner_ao[1], fract_pos.x);
    let ao_top = mix(corner_ao[2], corner_ao[3], fract_pos.x);
    let ao = mix(ao_bottom, ao_top, fract_pos.y);
    
    // Apply a power curve for better visual results
    return pow(ao, 1.5);
}

// Helper function to check if a voxel is occupied
fn is_voxel_occupied(pos: vec3<i32>) -> bool {
    // Convert to local chunk position for texture lookup
    let local_pos = pos - material.chunk_position;
    
    // Check bounds
    if (any(local_pos < vec3<i32>(0)) || any(local_pos >= vec3<i32>(64))) {
        return false;
    }
    
    // Sample the voxel texture
    let tex_coord = vec3<f32>(local_pos) / vec3<f32>(64.0);
    return textureSample(voxel_texture, voxel_sampler, tex_coord).r > 0.5;
}

@fragment
fn fragment(
    in: VertexOutput,
) -> FragmentOutput {
    // Calculate and apply AO factor
    var out: FragmentOutput;
    var pbr_input = pbr_input_from_standard_material(in, false);
    
    // Pass UV coordinates to the AO calculation
    let ao_factor = calculate_ao(in.world_position.xyz, in.world_normal, in.uv);
    
    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, in.color);
    out.color = apply_pbr_lighting(pbr_input);
    
    // Apply post-processing
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    out.color = apply_ambient_occlusion(out.color, ao_factor);
    
    return out;
}