use std::collections::BTreeSet;

use bevy::{
    image::Image,
    log::info_span,
    prelude::Mesh,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_asset::RenderAssetUsages,
        render_resource::{
            Extent3d, PrimitiveTopology, TextureDimension, TextureFormat, VertexFormat,
        },
    },
};
use binary_greedy_meshing as bgm;

use super::texture_array::TextureMapTrait;
use crate::world::{linearize, ChunkPos, CHUNKP_S1, CHUNK_S1};
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
pub const ATTRIBUTE_VOXEL_DATA: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelData", 48757581, VertexFormat::Uint32x2);

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

    /// Doesn't work with lod > 2, because chunks are of size 62 (to get to 64 with padding) and 62 = 2*31
    /// TODO: make it work with lod > 2 if necessary (by truncating quads)
    pub fn create_face_meshes(
        &self,
        texture_map: impl TextureMapTrait,
        lod: usize,
    ) -> [Option<Mesh>; 6] {
        // Gathering binary greedy meshing input data
        let mesh_data_span = info_span!("mesh voxel data", name = "mesh voxel data").entered();
        let voxels = self.voxel_data_lod(lod);
        let mut mesh_data = bgm::MeshData::new();
        mesh_data_span.exit();
        let mesh_build_span = info_span!("mesh build", name = "mesh build").entered();
        let transparents =
            BTreeSet::from_iter(self.palette.iter().enumerate().filter_map(|(i, block)| {
                if i != 0 && !block.is_opaque() {
                    Some(i as u16)
                } else {
                    None
                }
            }));
        bgm::mesh(&voxels, &mut mesh_data, transparents);
        let mut meshes = core::array::from_fn(|_| None);
        for (face_n, quads) in mesh_data.quads.iter().enumerate() {
            let mut voxel_data: Vec<[u32; 2]> = Vec::with_capacity(quads.len() * 4);
            let indices = bgm::indices(quads.len());
            let face: Face = face_n.into();
            for quad in quads {
                let voxel_i = (quad >> 32) as usize;
                let w = (MASK_6 & (quad >> 18)) as u32;
                let h = (MASK_6 & (quad >> 24)) as u32;
                let xyz = MASK_XYZ & quad;
                let block = self.palette[voxel_i];

                let color = match (block, face) {
                    (Block::GrassBlock, Face::Up) => 0b011_111_001,
                    (Block::SeaBlock, _) => 0b110_011_001,
                    (block, _) if block.is_foliage() => 0b010_101_001,
                    _ => 0b111_111_111,
                };
                let vertices = face.vertices_packed(xyz as u32, w as u32, h as u32, lod as u32);
                let quad_info = (w << 24) | (h << 18) | (color << 3) | face_n as u32;
                voxel_data.extend_from_slice(&[
                    [vertices[0], quad_info],
                    [vertices[1], quad_info],
                    [vertices[2], quad_info],
                    [vertices[3], quad_info],
                ]);
            }
            meshes[face_n] = Some(
                Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                )
                .with_inserted_attribute(ATTRIBUTE_VOXEL_DATA, voxel_data)
                .with_inserted_indices(Indices::U32(indices)),
            )
        }
        mesh_build_span.exit();
        meshes
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
