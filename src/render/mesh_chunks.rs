use std::{
    collections::{BTreeSet, HashMap},
    vec,
};

use bevy::{
    image::Image,
    log::info_span,
    math::Vec3,
    prelude::Mesh,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_asset::RenderAssetUsages,
        render_resource::{
            Extent3d, PrimitiveTopology, TextureDimension, TextureFormat, VertexFormat,
        },
    },
};
use binary_greedy_meshing::{self as bgm, Quad};

use super::{draw_chunks::ChunkMeshingState, texture_array::TextureMapTrait};
use crate::{
    block,
    render::draw_chunks::MeshingStage,
    utils::timeit_mut,
    world::{linearize, ChunkPos, CHUNKP_S1, CHUNK_S1},
};
use crate::{
    block::Face,
    world::{pad_linearize, Chunk, CHUNKP_S3},
    Block,
};

const MASK_6: u64 = 0b111111;
const MASK_XYZ: u64 = 0b111111_111111_111111;
/// ## Compressed voxel vertex data
/// first u32 (vertex dependant):
///     - chunk position: 3x6 bits (33 values)
///     - texture coords: 2x6 bits (33 values)
///     - ambiant occlusion?: 2 bits (4 values)
/// `0bao_vvvvvv_uuuuuu_zzzzzz_yyyyyy_xxxxxx`
///
/// second u32 (vertex agnostic):
///     - normals: 3 bits (6 values) = face
///     - color: 9 bits (3 r, 3 g, 3 b)
///     - texture layer: 16 bits
///     - light level: 4 bits (16 value)
///
/// `0bllll_iiiiiiiiiiiiiiii_ccccccccc_nnn`
pub const ATTRIBUTE_QUAD_SIZE: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelData", 48757581, VertexFormat::Float32x2);

impl Chunk {
    pub fn voxel_data_lod(&self, lod: usize) -> Vec<u16> {
        let voxels = self.data.unpack_u16();
        if lod == 1 {
            return voxels;
        }
        let mut res = vec![0; CHUNKP_S3];
        for x in 0..CHUNKP_S1 {
            for y in 0..CHUNKP_S1 {
                for z in 0..CHUNKP_S1 {
                    let xyz = linearize(x, y, z);
                    res[xyz] = voxels[xyz];
                }
            }
        }
        res
    }

    //TODO, break this to be iteratable over many frames
    pub fn create_face_meshes(
        &self,
        in_progress_state: &mut ChunkMeshingState,
    ) -> Option<(Mesh, Vec<[Vec3; 4]>)> {
        let lod = 1;
        // Gathering binary greedy meshing input data
        let mesh_data_span = info_span!("mesh voxel data", name = "mesh voxel data").entered();
        if in_progress_state.stage == MeshingStage::PrepareData {
            in_progress_state.voxels = self.voxel_data_lod(lod);
            in_progress_state.mesh_data = bgm::MeshData::new();
            in_progress_state.stage = MeshingStage::Transparents;
        }

        mesh_data_span.exit();
        if in_progress_state.stage == MeshingStage::Transparents {
            in_progress_state.transparents =
                BTreeSet::from_iter(self.palette.iter().enumerate().filter_map(|(i, block)| {
                    if i != 0 && !block.is_opaque() {
                        Some(i as u16)
                    } else {
                        None
                    }
                }));
            in_progress_state.stage = MeshingStage::GreedyMeshing;
        }
        if in_progress_state.stage == MeshingStage::GreedyMeshing {
            bgm::mesh(
                &in_progress_state.voxels,
                &mut in_progress_state.mesh_data,
                in_progress_state.transparents.clone(),
            );
            in_progress_state.stage = MeshingStage::ProcessQuads;
        }

        if in_progress_state.stage == MeshingStage::ProcessQuads {
            for (face_n, quads) in in_progress_state.mesh_data.quads.iter().enumerate() {
                if quads.is_empty() {
                    continue;
                }

                let mut physics_quads: Vec<[Vec3; 4]> = Vec::with_capacity(quads.len());

                let mut i = 0;
                for quad in quads {
                    i += 1;
                    let quad = *quad;
                    let voxel_i = quad.v_type as usize;
                    let block = self.palette[voxel_i];

                    // Get mesh data for this quad
                    let quad_mesh_data = quad_to_mesh_data(quad, block, face_n, i);

                    // Create a new set of indices for this quad
                    let mut quad_indices = Vec::with_capacity(4);

                    // Create physics quad vertices for collision detection
                    let mut physics_verts = [Vec3::ZERO; 4];

                    // Process each vertex of the quad
                    for i in 0..4 {
                        let position = quad_mesh_data.positions[i];
                        let normal = quad_mesh_data.normals[i];
                        let uv = quad_mesh_data.uvs[i];
                        let color = quad_mesh_data.colors[i];

                        // Convert floats to integers for hashing (with appropriate precision)
                        let pos_key = (
                            (position[0] * 1000.0) as i32,
                            (position[1] * 1000.0) as i32,
                            (position[2] * 1000.0) as i32,
                        );
                        let normal_key = (
                            (normal[0] * 100.0) as i8,
                            (normal[1] * 100.0) as i8,
                            (normal[2] * 100.0) as i8,
                        );
                        let uv_key = ((uv[0] * 100.0) as i16, (uv[1] * 100.0) as i16);
                        let color_key = (
                            (color[0] * 255.0) as u8,
                            (color[1] * 255.0) as u8,
                            (color[2] * 255.0) as u8,
                            (color[3] * 255.0) as u8,
                        );

                        // Create a unique key for this vertex
                        let vertex_key = (pos_key, normal_key, uv_key, color_key);

                        // Get or create the vertex index
                        let vertex_index = match in_progress_state.vertex_map.get(&vertex_key) {
                            Some(&index) => index,
                            None => {
                                // New unique vertex
                                let index = in_progress_state.next_vertex_index;
                                in_progress_state.vertex_map.insert(vertex_key, index);

                                // Add vertex data to the combined arrays
                                in_progress_state.all_positions.push(position);
                                in_progress_state.all_normals.push(normal);
                                in_progress_state.all_uvs.push(uv);
                                in_progress_state.all_colors.push(color);
                                // all_quad_sizes.push(quad_mesh_data.quad_sizes);

                                in_progress_state.next_vertex_index += 1;
                                index
                            }
                        };

                        // Store the vertex index for this quad
                        quad_indices.push(vertex_index);

                        // Store physics vertex
                        physics_verts[i] = Vec3::new(position[0], position[1], position[2]);
                    }

                    // Add the indices for this quad (two triangles)
                    // Adjust the order based on the face type
                    match face_n {
                        0 => {
                            // Face::Up
                            in_progress_state.all_indices.extend_from_slice(&[
                                quad_indices[2] as u32,
                                quad_indices[0] as u32,
                                quad_indices[1] as u32,
                                quad_indices[2] as u32,
                                quad_indices[3] as u32,
                                quad_indices[0] as u32,
                            ]);
                        }
                        1 => {
                            // Face::Down
                            in_progress_state.all_indices.extend_from_slice(&[
                                quad_indices[0] as u32,
                                quad_indices[2] as u32,
                                quad_indices[1] as u32,
                                quad_indices[0] as u32,
                                quad_indices[3] as u32,
                                quad_indices[2] as u32,
                            ]);
                        }
                        2 => {
                            // Face::Right
                            in_progress_state.all_indices.extend_from_slice(&[
                                quad_indices[1] as u32,
                                quad_indices[0] as u32,
                                quad_indices[2] as u32,
                                quad_indices[0] as u32,
                                quad_indices[3] as u32,
                                quad_indices[2] as u32,
                            ]);
                        }
                        3 => {
                            // Face::Left
                            in_progress_state.all_indices.extend_from_slice(&[
                                quad_indices[0] as u32,
                                quad_indices[1] as u32,
                                quad_indices[3] as u32,
                                quad_indices[3] as u32,
                                quad_indices[1] as u32,
                                quad_indices[2] as u32,
                            ]);
                        }
                        4 => {
                            // Face::Front
                            in_progress_state.all_indices.extend_from_slice(&[
                                quad_indices[1] as u32,
                                quad_indices[0] as u32,
                                quad_indices[2] as u32,
                                quad_indices[2] as u32,
                                quad_indices[0] as u32,
                                quad_indices[3] as u32,
                            ]);
                        }
                        5 => {
                            // Face::Back
                            in_progress_state.all_indices.extend_from_slice(&[
                                quad_indices[1] as u32,
                                quad_indices[2] as u32,
                                quad_indices[0] as u32,
                                quad_indices[2] as u32,
                                quad_indices[3] as u32,
                                quad_indices[0] as u32,
                            ]);
                        }
                        _ => {}
                    }

                    physics_quads.push(physics_verts);
                }

                in_progress_state.all_physics_quads.extend(physics_quads);
            }
            in_progress_state.stage = MeshingStage::Finalize;
        }
        if in_progress_state.stage == MeshingStage::Finalize {
            in_progress_state.stage = MeshingStage::Complete;
            // If we have no vertices, return None
            if in_progress_state.all_positions.is_empty() {
                in_progress_state.is_empty = true;
                return None;
            }

            // Create the combined render mesh with standard attributes
            let mut render_mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            );
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                in_progress_state.all_positions.clone(),
            );
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_NORMAL,
                in_progress_state.all_normals.clone(),
            );
            render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, in_progress_state.all_uvs.clone());
            render_mesh
                .insert_attribute(Mesh::ATTRIBUTE_COLOR, in_progress_state.all_colors.clone());
            // render_mesh.insert_attribute(ATTRIBUTE_QUAD_SIZE, all_quad_sizes);
            render_mesh.insert_indices(Indices::U32(in_progress_state.all_indices.clone()));
            return Some((render_mesh, in_progress_state.all_physics_quads.clone()));
        }
        None
    }

    pub fn create_ao_texture_data(&self) -> Image {
        let dim = CHUNKP_S1;

        // Calculate how many u32s we need (CHUNKP_S3 / 32, rounded up)
        let u32_count = (CHUNKP_S3 + 31) / 32;
        let mut texture_data = vec![0u32; u32_count];

        let voxels = self.data.unpack_u16();

        for y in 0..dim {
            for z in 0..dim {
                for x in 0..dim {
                    let xyz = linearize(x, y, z);

                    if voxels[xyz] != 0 {
                        // Calculate which u32 and which bit within that u32
                        let u32_index = xyz / 32;
                        let bit_position = xyz % 32;

                        // Set the appropriate bit
                        texture_data[u32_index] |= 1 << bit_position;
                    }
                }
            }
        }

        // Convert u32 array to bytes using bytemuck
        let bytes = bytemuck::cast_slice(&texture_data).to_vec();

        // Need to adjust dimensions to account for the packing
        let width = (dim + 31) / 32; // Width in terms of u32s

        Image::new(
            Extent3d {
                width: width as u32,
                height: dim as u32,
                depth_or_array_layers: dim as u32,
            },
            TextureDimension::D3,
            bytes,
            TextureFormat::R32Uint, //R8Uint
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}
pub struct QuadMeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    quad_sizes: [f32; 2],
}
pub fn quad_to_mesh_data(quad: Quad, block: Block, face_n: usize, quad_index: u32) -> QuadMeshData {
    // Extract components
    let x = (quad.x as f32) / 8.0;
    let y = (quad.y as f32) / 8.0;
    let z = (quad.z as f32) / 8.0;
    let w = (quad.w as f32) / 8.0;
    let h = (quad.h as f32) / 8.0;
    let face: Face = face_n.into();

    let normal = match face {
        Face::Up => [0.0, 1.0, 0.0],    // Up
        Face::Down => [0.0, -1.0, 0.0], // Down
        Face::Right => [1.0, 0.0, 0.0], // Right
        Face::Left => [-1.0, 0.0, 0.0], // Left
        Face::Front => [0.0, 0.0, 1.0], // Front
        Face::Back => [0.0, 0.0, -1.0], // Back
        _ => [0.0, 0.0, 0.0],           // Shouldn't happen
    };

    // Generate positions based on face orientation
    let positions = match face {
        Face::Up => {
            vec![[x, y, z], [x, y, z + h], [x + w, y, z + h], [x + w, y, z]]
        }
        Face::Down => {
            vec![[x, y, z], [x + w, y, z], [x + w, y, z + h], [x, y, z + h]]
        }
        Face::Right => {
            vec![[x, y, z], [x, y - w, z], [x, y - w, z + h], [x, y, z + h]]
        }
        Face::Left => {
            vec![[x, y, z], [x, y, z + h], [x, y + w, z + h], [x, y + w, z]]
        }
        Face::Front => {
            vec![[x, y, z], [x - w, y, z], [x - w, y + h, z], [x, y + h, z]]
        }
        Face::Back => {
            vec![[x, y, z], [x, y + h, z], [x + w, y + h, z], [x + w, y, z]]
        }
    };
    // Generate UVs (simple 0-1 mapping)
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    // Generate normals (same for all vertices of the quad)
    let normals = vec![normal; 4];
    let colors = vec![get_color_from_block(&block, &face); 4];

    QuadMeshData {
        positions,
        normals,
        uvs,
        colors,
        quad_sizes: [w, h],
    }
}

fn get_indices(face: Face, quad_index: u32) -> Vec<u32> {
    let base = quad_index << 2; // Multiply by 4 to get the base vertex index

    match face {
        Face::Up => vec![base + 2, base + 0, base + 1, base + 2, base + 3, base + 0],
        Face::Down => vec![base + 0, base + 2, base + 1, base + 0, base + 3, base + 2],
        Face::Front => vec![base + 1, base + 0, base + 2, base + 2, base + 0, base + 3],
        Face::Back => vec![base + 1, base + 2, base + 0, base + 2, base + 3, base + 0],
        Face::Left => vec![base + 0, base + 1, base + 3, base + 3, base + 1, base + 2],
        Face::Right => vec![base + 1, base + 0, base + 2, base + 0, base + 3, base + 2],
    }
}

pub fn get_color_from_block(block: &Block, face: &Face) -> [f32; 4] {
    let color_bits = match (block, face) {
        (Block::GrassBlock, Face::Up) => 0b011_111_001,
        (Block::SeaBlock, _) => 0b110_011_001,
        (block, _) if block.is_foliage() => 0b010_101_001,
        _ => 0b111_111_111,
    };

    // Convert color bits to RGB float values
    let r = ((color_bits >> 6) & 0x7) as f32 / 7.0;
    let g = ((color_bits >> 3) & 0x7) as f32 / 7.0;
    let b = (color_bits & 0x7) as f32 / 7.0;
    [r, g, b, 1.0]
}
