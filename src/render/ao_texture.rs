use bevy::{
    asset::{Handle, RenderAssetUsages},
    ecs::system::{Commands, Resource},
    image::{Image, ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{AddressMode, Extent3d, TextureDimension, TextureFormat},
    utils::HashMap,
};

use crate::block::Face;

/// Configuration for ambient occlusion textures
#[derive(Resource)]
pub struct AOTextureConfig {
    /// Resolution of AO textures (width/height in pixels)
    pub resolution: u32,
    /// Strength of the AO effect (0.0 to 1.0)
    pub strength: f32,
    /// How far the AO effect extends from corners/edges
    pub length: f32,
}

impl Default for AOTextureConfig {
    fn default() -> Self {
        Self {
            resolution: 256,
            strength: 1.0,
            length: 0.6,
        }
    }
}

/// Represents different AO patterns that can be applied to faces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AOPattern {
    None,
    OneCorner,
    TwoCorners,
    TwoOppositeCorners,
    ThreeCorners,
    FourCorners,
    OneEdge,
    OppositeEdges,
    TwoAdjacentEdges,
    ThreeEdges,
    FourEdges,
}

/// Stores handles to generated AO textures
#[derive(Resource)]
pub struct AOTextureArray {
    // Maps pattern and rotation (0-3) to texture handle
    pub textures: HashMap<AOPattern, Handle<Image>>,
}

/// Generates ambient occlusion textures based on configuration
pub fn generate_ao_textures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    config: Res<AOTextureConfig>,
) {
    let mut ao_textures = HashMap::new();
    let resolution = config.resolution as usize;
    let strength = config.strength;
    let length = config.length;

    // Generate each AO pattern
    for pattern in [
        AOPattern::None,
        AOPattern::OneCorner,
        AOPattern::TwoCorners,
        AOPattern::TwoOppositeCorners,
        AOPattern::ThreeCorners,
        AOPattern::FourCorners,
        AOPattern::OneEdge,
        AOPattern::OppositeEdges,
        AOPattern::TwoAdjacentEdges,
        AOPattern::ThreeEdges,
        AOPattern::FourEdges,
    ] {
        let mut texture_data = vec![255u8; resolution * resolution * 4];

        for y in 0..resolution {
            for x in 0..resolution {
                let nx = x as f32 / resolution as f32;
                let ny = y as f32 / resolution as f32;

                let ao_value = match pattern {
                    AOPattern::None => 1.0,
                    AOPattern::OneCorner => calculate_corner_ao(nx, ny, 0.0, 0.0, length),
                    AOPattern::TwoCorners => {
                        calculate_corner_ao(nx, ny, 0.0, 0.0, length)
                            * calculate_corner_ao(nx, ny, 1.0, 0.0, length)
                    }
                    AOPattern::TwoOppositeCorners => {
                        calculate_corner_ao(nx, ny, 0.0, 0.0, length)
                            * calculate_corner_ao(nx, ny, 1.0, 1.0, length)
                    }
                    AOPattern::ThreeCorners => {
                        calculate_corner_ao(nx, ny, 0.0, 0.0, length)
                            * calculate_corner_ao(nx, ny, 1.0, 0.0, length)
                            * calculate_corner_ao(nx, ny, 0.0, 1.0, length)
                    }
                    AOPattern::FourCorners => {
                        calculate_corner_ao(nx, ny, 0.0, 0.0, length)
                            * calculate_corner_ao(nx, ny, 1.0, 0.0, length)
                            * calculate_corner_ao(nx, ny, 0.0, 1.0, length)
                            * calculate_corner_ao(nx, ny, 1.0, 1.0, length)
                    }
                    AOPattern::OneEdge => {
                        calculate_edge_ao(nx, ny, true, false, false, false, length)
                    }
                    AOPattern::OppositeEdges => {
                        calculate_edge_ao(nx, ny, true, false, true, false, length)
                    }
                    AOPattern::TwoAdjacentEdges => {
                        calculate_edge_ao(nx, ny, true, true, false, false, length)
                    }
                    AOPattern::ThreeEdges => {
                        calculate_edge_ao(nx, ny, true, true, true, false, length)
                    }
                    AOPattern::FourEdges => {
                        calculate_edge_ao(nx, ny, true, true, true, true, length)
                    }
                };

                // Apply strength and convert to byte
                let ao_byte = ((1.0 - ((1.0 - ao_value) * strength)) * 255.0) as u8;

                let index = (y * resolution + x) * 4;
                texture_data[index] = ao_byte; // R
                texture_data[index + 1] = ao_byte; // G
                texture_data[index + 2] = ao_byte; // B
                texture_data[index + 3] = 255; // A (fully opaque)
            }
        }

        let mut image = Image::new(
            Extent3d {
                width: resolution as u32,
                height: resolution as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            texture_data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );

        // Set proper sampler for the texture
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            address_mode_w: ImageAddressMode::ClampToEdge,
            ..Default::default()
        });

        let handle = images.add(image);
        ao_textures.insert(pattern, handle);
    }
    
    commands.insert_resource(AOTextureArray {
        textures: ao_textures,
    });
}

/// Calculate AO value for a corner
fn calculate_corner_ao(x: f32, y: f32, corner_x: f32, corner_y: f32, length: f32) -> f32 {
    let dist = ((x - corner_x).powi(2) + (y - corner_y).powi(2)).sqrt();
    (dist / length).min(1.0)
}

/// Calculate AO value for edges
fn calculate_edge_ao(
    x: f32,
    y: f32,
    left: bool,
    top: bool,
    right: bool,
    bottom: bool,
    length: f32,
) -> f32 {
    let mut ao: f32 = 1.0;

    if left {
        ao = ao.min(x / length);
    }
    if right {
        ao = ao.min((1.0 - x) / length);
    }
    if top {
        ao = ao.min(y / length);
    }
    if bottom {
        ao = ao.min((1.0 - y) / length);
    }

    ao.min(1.0)
}

/// Add this plugin to your app to set up AO textures
pub struct AOTexturePlugin;

impl Plugin for AOTexturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AOTextureConfig {
            resolution: 512,
            strength: 0.7,
            length: 0.25,
            ..Default::default()
        })
        .add_systems(Startup, generate_ao_textures);
    }
}
/// Determines the appropriate AO texture pattern and rotation for a voxel face
///
/// # Arguments
/// * [face](cci:1://file:///d:/Personal%20Projects/riverbed/src/render/chunk_culling.rs:69:0-80:1) - The face direction
/// * [index](cci:1://file:///d:/Personal%20Projects/riverbed/src/render/texture_array.rs:37:4-48:5) - The index of the voxel in the voxels array
/// * `voxels` - The array of voxel data
/// * `cs_p` - The chunk size plus padding (CS_P)
///
/// # Returns
/// A tuple containing:
/// * `AOPattern` - The ambient occlusion pattern to use
/// * `u8` - Rotation in 90-degree increments (0-3)
pub fn determine_ao_pattern(
    face: Face,
    index: usize,
    voxels: &[u16],
    cs_p: usize,
) -> (AOPattern, u8) {
    // Constants for neighbor offsets
    let cs_p2 = cs_p * cs_p;
    let get_voxel = |offset: isize| -> u16 {
        if offset >= 0 {
            voxels
                .get(index.wrapping_add(offset as usize))
                .copied()
                .unwrap_or(0)
        } else {
            // Handle negative offsets safely
            if index >= ((-offset) as usize) {
                voxels
                    .get(index - ((-offset) as usize))
                    .copied()
                    .unwrap_or(0)
            } else {
                0 // Out of bounds
            }
        }
    };

    // Get offsets for the corners and edges around the face
    // These are relative to the current voxel and depend on the face direction
    let (corner_offsets, edge_offsets) = match face {
        Face::Right => {
            // +X face
            (
                [
                    1 + cs_p as isize - cs_p2 as isize,
                    1 + cs_p as isize,
                    1 - cs_p2 as isize,
                    1,
                ],
                [
                    1 + cs_p as isize,  // top
                    1 + cs_p2 as isize, // right
                    1 - cs_p as isize,  // bottom
                    1 - cs_p2 as isize, // left
                ],
            )
        }
        Face::Left => {
            // -X face
            (
                [
                    -1 - cs_p as isize - cs_p2 as isize,
                    -1 - cs_p2 as isize,
                    -1 - cs_p2 as isize + cs_p as isize,
                    -1 + cs_p as isize - cs_p2 as isize,
                ],
                [
                    -1 - cs_p as isize,  // top
                    -1 - cs_p2 as isize, // right
                    -1 + cs_p as isize,  // bottom
                    -1 + cs_p2 as isize, // left
                ],
            )
        }
        Face::Up => {
            // +Y face
            (
                [
                    cs_p as isize - 1 - cs_p2 as isize,
                    cs_p as isize - 1,
                    cs_p as isize + 1 - cs_p2 as isize,
                    cs_p as isize + 1,
                ],
                [
                    cs_p as isize - 1,              // top
                    cs_p as isize - cs_p2 as isize, // right
                    cs_p as isize + 1,              // bottom
                    cs_p as isize + cs_p2 as isize, // left
                ],
            )
        }
        Face::Down => {
            // -Y face
            (
                [
                    -(cs_p as isize) - 1 - cs_p2 as isize,
                    -(cs_p as isize) - 1,
                    -(cs_p as isize) + 1 - cs_p2 as isize,
                    -(cs_p as isize) + 1,
                ],
                [
                    -(cs_p as isize) - 1,              // top
                    -(cs_p as isize) - cs_p2 as isize, // right
                    -(cs_p as isize) + 1,              // bottom
                    -(cs_p as isize) + cs_p2 as isize, // left
                ],
            )
        }
        Face::Front => {
            // +Z face
            (
                [
                    1 - cs_p as isize - cs_p2 as isize,
                    1 - cs_p2 as isize,
                    1 + cs_p as isize - cs_p2 as isize,
                    1 + cs_p as isize,
                ],
                [
                    1 - cs_p2 as isize, // top
                    1 + cs_p as isize,  // right
                    1 + cs_p2 as isize, // bottom
                    1 - cs_p as isize,  // left
                ],
            )
        }
        Face::Back => {
            // -Z face
            (
                [
                    -1 - cs_p as isize - cs_p2 as isize,
                    -1 - cs_p2 as isize,
                    -1 + cs_p as isize - cs_p2 as isize,
                    -1 + cs_p as isize,
                ],
                [
                    -1 - cs_p2 as isize, // top
                    -1 - cs_p as isize,  // right
                    -1 + cs_p2 as isize, // bottom
                    -1 + cs_p as isize,  // left
                ],
            )
        }
    };
    // Check which corners are solid (we only need the 4 corners around the face)
    let corner_occupied = [
        get_voxel(corner_offsets[0]) != 0,
        get_voxel(corner_offsets[1]) != 0,
        get_voxel(corner_offsets[2]) != 0,
        get_voxel(corner_offsets[3]) != 0,
    ];

    // Check which edges are solid
    let edge_occupied = [
        get_voxel(edge_offsets[0]) != 0,
        get_voxel(edge_offsets[1]) != 0,
        get_voxel(edge_offsets[2]) != 0,
        get_voxel(edge_offsets[3]) != 0,
    ];

    // Count occupied corners and edges
    let corner_count = corner_occupied.iter().filter(|&&b| b).count();
    let edge_count = edge_occupied.iter().filter(|&&b| b).count();

    // First, handle special case where there are no occupied corners or edges
    if corner_count == 0 && edge_count == 0 {
        return (AOPattern::None, 0);
    }

    // Handle edge-only cases
    if corner_count == 0 {
        match edge_count {
            1 => {
                // One edge - determine rotation
                let rotation = if edge_occupied[0] {
                    0
                } else if edge_occupied[1] {
                    1
                } else if edge_occupied[2] {
                    2
                } else {
                    3
                };
                return (AOPattern::OneEdge, rotation);
            }
            2 => {
                // Check if opposite edges
                if (edge_occupied[0] && edge_occupied[2]) || (edge_occupied[1] && edge_occupied[3])
                {
                    let rotation = if edge_occupied[0] && edge_occupied[2] {
                        0
                    } else {
                        1
                    };
                    return (AOPattern::OppositeEdges, rotation);
                } else {
                    // Adjacent edges
                    let rotation = if edge_occupied[0] && edge_occupied[1] {
                        0
                    } else if edge_occupied[1] && edge_occupied[2] {
                        1
                    } else if edge_occupied[2] && edge_occupied[3] {
                        2
                    } else {
                        3
                    };
                    return (AOPattern::TwoAdjacentEdges, rotation);
                }
            }
            3 => {
                // Three edges
                let rotation = if !edge_occupied[0] {
                    2
                } else if !edge_occupied[1] {
                    3
                } else if !edge_occupied[2] {
                    0
                } else {
                    1
                };
                return (AOPattern::ThreeEdges, rotation);
            }
            4 => {
                // All edges
                return (AOPattern::FourEdges, 0);
            }
            _ => unreachable!(),
        }
    }

    // Handle corner cases
    match corner_count {
        1 => {
            // One corner
            let rotation = if corner_occupied[0] {
                0
            } else if corner_occupied[1] {
                1
            } else if corner_occupied[2] {
                2
            } else {
                3
            };
            return (AOPattern::OneCorner, rotation);
        }
        2 => {
            // Check if opposite corners
            if (corner_occupied[0] && corner_occupied[3])
                || (corner_occupied[1] && corner_occupied[2])
            {
                let rotation = if corner_occupied[0] && corner_occupied[3] {
                    0
                } else {
                    1
                };
                return (AOPattern::TwoOppositeCorners, rotation);
            } else {
                // Adjacent corners
                let rotation = if corner_occupied[0] && corner_occupied[1] {
                    0
                } else if corner_occupied[1] && corner_occupied[3] {
                    1
                } else if corner_occupied[2] && corner_occupied[3] {
                    2
                } else {
                    3
                };
                return (AOPattern::TwoCorners, rotation);
            }
        }
        3 => {
            // Three corners
            let rotation = if !corner_occupied[0] {
                2
            } else if !corner_occupied[1] {
                3
            } else if !corner_occupied[2] {
                0
            } else {
                1
            };
            return (AOPattern::ThreeCorners, rotation);
        }
        4 => {
            // All corners
            return (AOPattern::FourCorners, 0);
        }
        _ => unreachable!(),
    }
}
