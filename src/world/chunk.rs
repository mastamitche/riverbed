use super::{
    pos::{ChunkedPos, ColedPos},
    utils::Palette,
    ChunkPos, CHUNKP_S1, CHUNKP_S2, CHUNKP_S3, CHUNK_S1,
};
use crate::{render::texture_array::VoxelChunkMaterial, Block};
use bevy::{
    asset::{Assets, Handle, RenderAssetUsages},
    image::{Image, ImageSampler},
    math::{IVec3, UVec3},
    pbr::{ExtendedMaterial, StandardMaterial},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use itertools::Itertools;
use packed_uints::PackedUints;

#[derive(Debug)]
pub struct Chunk {
    pub data: PackedUints,
    pub palette: Palette<Block>,
    pub packed_ao_mask: Vec<u8>,
    pub ao_texture_handle: Option<Handle<Image>>,
    ao_dirty: bool,
}
const AO_MASK_SIZE: usize = CHUNKP_S1 * CHUNKP_S1 * CHUNKP_S1; // 8 * 64 * 64 = 32,768

pub fn pad_linearize(x: usize, y: usize, z: usize) -> usize {
    // Remove the +1 offsets to keep indices within bounds
    z + x * CHUNKP_S1 + y * CHUNKP_S2
}

impl Chunk {
    pub fn get(&self, (x, y, z): ChunkedPos) -> &Block {
        &self.palette[self.data.get(pad_linearize(x, y, z))]
    }

    pub fn set(&mut self, (x, y, z): ChunkedPos, block: Block) {
        let idx = pad_linearize(x, y, z);
        self.data.set(idx, self.palette.index(block));
        self.ao_dirty = true;
    }

    pub fn set_yrange(&mut self, (x, top, z): ChunkedPos, height: usize, block: Block) {
        let value = self.palette.index(block);
        // Note: we do end+1 because set_range(_step) is not inclusive
        self.data.set_range_step(
            pad_linearize(x, top - height, z),
            pad_linearize(x, top, z) + 1,
            CHUNKP_S2,
            value,
        );
        self.ao_dirty = true;
    }
    #[allow(dead_code)]
    pub fn copy_column(&self, buffer: &mut [Block], (x, z): ColedPos, lod: usize) {
        let start = pad_linearize(x, 0, z);
        let mut i = 0;
        for idx in (start..(start + CHUNK_S1)).step_by(lod) {
            buffer[i] = self.palette[self.data.get(idx)];
            i += 1;
        }
    }

    pub fn top(&self, (x, z): ColedPos) -> (&Block, usize) {
        for y in (0..CHUNK_S1).rev() {
            let b_idx = self.data.get(pad_linearize(x, y, z));
            if b_idx > 0 {
                return (&self.palette[b_idx], y);
            }
        }
        (&self.palette[0], 0)
    }

    pub fn set_if_empty(&mut self, (x, y, z): ChunkedPos, block: Block) -> bool {
        let idx = pad_linearize(x, y, z);
        if self.palette[self.data.get(idx)] != Block::Air {
            return false;
        }
        self.data.set(idx, self.palette.index(block));
        self.ao_dirty = true;
        true
    }

    pub fn create_ao_textures_for_meshes(
        &mut self,
        images: &mut Assets<Image>,
        chunk_pos: ChunkPos,
        mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, VoxelChunkMaterial>>,
        material_handle: &Handle<ExtendedMaterial<StandardMaterial, VoxelChunkMaterial>>,
    ) {
        if self.ao_dirty {
            let has_existing_handle = self.ao_texture_handle.is_some();

            let texture_handle = if has_existing_handle {
                let existing_handle = self.ao_texture_handle.as_ref().unwrap().clone();
                self.update_ao_texture(images, &existing_handle)
            } else {
                let handle = self.create_ao_texture(images);
                self.ao_texture_handle = Some(handle.clone());
                handle
            };

            if let Some(material) = mesh_materials.get_mut(material_handle) {
                material.extension.voxel_data = texture_handle;

                material.extension.chunk_data.chunk_position =
                    IVec3::new(chunk_pos.x, chunk_pos.y, chunk_pos.z) * CHUNK_S1 as i32;
            }

            self.ao_dirty = false;
        }
    }
    fn create_ao_texture(&mut self, images: &mut Assets<Image>) -> Handle<Image> {
        // No packing - use full resolution texture
        const TEX_WIDTH: usize = CHUNKP_S1;
        const TEX_HEIGHT: usize = CHUNKP_S1;
        const TEX_DEPTH: usize = CHUNKP_S1;
        let texture_size = UVec3::new(TEX_WIDTH as u32, TEX_HEIGHT as u32, TEX_DEPTH as u32);

        // Create full-sized voxel data buffer
        let texture_size_total = TEX_WIDTH * TEX_HEIGHT * TEX_DEPTH;
        let mut voxel_data = vec![0u8; texture_size_total];

        // Fill voxel data - 1 for solid, 0 for air
        for z in 0..CHUNKP_S1 {
            for y in 0..CHUNKP_S1 {
                for x in 0..CHUNKP_S1 {
                    let block_idx = self.data.get(pad_linearize(x, y, z));
                    let is_solid = self.palette[block_idx] != Block::Air;

                    // Set texel value (255 for solid, 0 for air)
                    let idx = x + y * TEX_WIDTH + z * TEX_WIDTH * TEX_HEIGHT;
                    voxel_data[idx] = if is_solid { 255 } else { 0 };
                }
            }
        }

        let mut image = Image::new(
            Extent3d {
                width: texture_size.x,
                height: texture_size.y,
                depth_or_array_layers: texture_size.z,
            },
            TextureDimension::D3,
            voxel_data,
            TextureFormat::R8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();

        images.add(image)
    }

    pub fn update_ao_texture(
        &mut self,
        images: &mut Assets<Image>,
        texture_handle: &Handle<Image>,
    ) -> Handle<Image> {
        if let Some(image) = images.get_mut(texture_handle) {
            // No packing - use full resolution texture
            const TEX_WIDTH: usize = CHUNKP_S1;
            const TEX_HEIGHT: usize = CHUNKP_S1;
            const TEX_DEPTH: usize = CHUNKP_S1;

            // Create or resize the buffer if needed
            let texture_size_total = TEX_WIDTH * TEX_HEIGHT * TEX_DEPTH;
            if image.data.len() != texture_size_total {
                image.data = vec![0u8; texture_size_total];
            }

            // Fill voxel data - 1 for solid, 0 for air
            for z in 0..CHUNKP_S1 {
                for y in 0..CHUNKP_S1 {
                    for x in 0..CHUNKP_S1 {
                        let block_idx = self.data.get(pad_linearize(x, y, z));
                        let is_solid = self.palette[block_idx] != Block::Air;

                        // Set texel value (255 for solid, 0 for air)
                        let idx = x + y * TEX_WIDTH + z * TEX_WIDTH * TEX_HEIGHT;
                        image.data[idx] = if is_solid { 255 } else { 0 };
                    }
                }
            }

            texture_handle.clone()
        } else {
            self.create_ao_texture(images)
        }
    }

    #[allow(dead_code)]
    pub fn is_voxel_present(&self, pos: ChunkedPos) -> bool {
        let block_idx = self.data.get(pad_linearize(pos.0, pos.1, pos.2));
        self.palette[block_idx] != Block::Air
    }
    /// Pre-computes the AO mask for the entire chunk
    pub fn precompute_ao_mask(&mut self) {
        const TEX_WIDTH: usize = CHUNKP_S1;
        const TEX_HEIGHT: usize = CHUNKP_S1;
        const TEX_DEPTH: usize = CHUNKP_S1;

        // Ensure our AO mask has the right size
        self.packed_ao_mask = vec![0u8; TEX_WIDTH * TEX_HEIGHT * TEX_DEPTH];

        // Fill voxel presence data
        for z in 0..CHUNKP_S1 {
            for y in 0..CHUNKP_S1 {
                for x in 0..CHUNKP_S1 {
                    let idx = pad_linearize(x, y, z);
                    if idx < self.data.length {
                        let block_idx = self.data.get(idx);
                        let is_solid = self.palette[block_idx] != Block::Air;

                        // Set texel value (255 for solid, 0 for air)
                        let tex_idx = x + y * TEX_WIDTH + z * TEX_WIDTH * TEX_HEIGHT;
                        self.packed_ao_mask[tex_idx] = if is_solid { 255 } else { 0 };
                    }
                }
            }
        }
    }
}

impl From<&[Block]> for Chunk {
    fn from(values: &[Block]) -> Self {
        let mut palette = Palette::new();
        palette.index(Block::Air);
        let values = values.iter().map(|v| palette.index(*v)).collect_vec();
        let data = PackedUints::from(values.as_slice());
        let mut chunk = Chunk {
            data,
            palette,
            packed_ao_mask: vec![0; AO_MASK_SIZE],
            ao_texture_handle: None,
            ao_dirty: true,
        };
        chunk.precompute_ao_mask();
        chunk
    }
}

impl Chunk {
    pub fn new() -> Self {
        let mut palette = Palette::new();
        palette.index(Block::Air);
        let mut chunk = Chunk {
            data: PackedUints::new(CHUNKP_S3),
            palette,
            packed_ao_mask: vec![0; AO_MASK_SIZE],
            ao_texture_handle: None,
            ao_dirty: true,
        };
        chunk.precompute_ao_mask();
        chunk
    }
}
