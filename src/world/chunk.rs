use crate::block::Block;

use super::{
    pos::{ChunkedPos, ColedPos},
    utils::Palette,
    CHUNKP_S1, CHUNKP_S2, CHUNKP_S3, CHUNK_S1,
};
use itertools::Itertools;
use packed_uints::{PackedEnum, PackedUints};

#[derive(Debug)]
pub struct Chunk {
    pub data: PackedUints,
    pub palette: Palette<Block>,
}

pub fn linearize(x: usize, y: usize, z: usize) -> usize {
    z + x * CHUNKP_S1 + y * CHUNKP_S2
}

pub fn pad_linearize(x: usize, y: usize, z: usize) -> usize {
    z + 1 + (x + 1) * CHUNKP_S1 + (y + 1) * CHUNKP_S2
}

impl Chunk {
    pub fn get(&self, (x, y, z): ChunkedPos) -> &Block {
        &self.palette[self.data.get(pad_linearize(x, y, z))]
    }

    pub fn set_no_padding(&mut self, (x, y, z): ChunkedPos, block: Block) {
        let idx = linearize(x, y, z);
        // if idx < 100 {
        //     println!("setting idx {} to {}", idx, self.palette.index(block));
        // }
        self.data.set(idx, self.palette.index(block));
    }
    pub fn set(&mut self, (x, y, z): ChunkedPos, block: Block) {
        let idx = pad_linearize(x, y, z);
        self.data.set(idx, self.palette.index(block));
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
    }

    // Used for efficient construction of mesh data
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
        true
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Serialize palette first
        let palette_blocks = self.palette.get_all_elements();
        buffer.extend_from_slice(&(palette_blocks.len() as u32).to_le_bytes());

        for block in &palette_blocks {
            // Serialize each block (assuming Block implements serialization)
            // This depends on how your Block is defined
            let block_id = block.to_id(); // Convert block to some numeric ID
            buffer.extend_from_slice(&block_id.to_le_bytes());
        }

        // Then serialize the packed data
        match &self.data.data {
            PackedEnum::U4(data) => {
                buffer.push(0); // Type identifier: 0 for U4
                buffer.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buffer.extend_from_slice(data);
            }
            PackedEnum::U8(data) => {
                buffer.push(1); // Type identifier: 1 for U8
                buffer.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buffer.extend_from_slice(data);
            }
            PackedEnum::U16(data) => {
                buffer.push(2); // Type identifier: 2 for U16
                buffer.extend_from_slice(&(data.len() as u32).to_le_bytes());
                for value in data {
                    buffer.extend_from_slice(&value.to_le_bytes());
                }
            }
            PackedEnum::U32(data) => {
                buffer.push(3); // Type identifier: 3 for U32
                buffer.extend_from_slice(&(data.len() as u32).to_le_bytes());
                for value in data {
                    buffer.extend_from_slice(&value.to_le_bytes());
                }
            }
        }

        buffer.extend_from_slice(&(self.data.mask as u32).to_le_bytes());
        buffer.extend_from_slice(&(self.data.length as u32).to_le_bytes());

        buffer
    }

    // Deserialize a chunk from bytes
    pub fn deserialize(bytes: &[u8]) -> Self {
        let mut cursor = 0;

        // Read palette size
        let palette_size = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;

        // Create palette
        let mut palette = Palette::new();
        for _ in 0..palette_size {
            let block_id = u32::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
            ]);
            cursor += 4;

            // Convert block_id back to Block
            let block = Block::from_id(block_id);
            palette.index(block);
        }

        // Read packed data type
        let packed_type = bytes[cursor];
        cursor += 1;

        // Read data length
        let data_length = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;

        // Read data based on type
        let data = match packed_type {
            0 => {
                // U4
                let data_slice = &bytes[cursor..cursor + data_length];
                cursor += data_length;
                PackedEnum::U4(data_slice.to_vec())
            }
            1 => {
                // U8
                let data_slice = &bytes[cursor..cursor + data_length];
                cursor += data_length;
                PackedEnum::U8(data_slice.to_vec())
            }
            2 => {
                // U16
                let mut data = Vec::with_capacity(data_length);
                for i in 0..data_length {
                    let idx = cursor + i * 2;
                    let value = u16::from_le_bytes([bytes[idx], bytes[idx + 1]]);
                    data.push(value);
                }
                cursor += data_length * 2;
                PackedEnum::U16(data)
            }
            3 => {
                // U32
                let mut data = Vec::with_capacity(data_length);
                for i in 0..data_length {
                    let idx = cursor + i * 4;
                    let value = u32::from_le_bytes([
                        bytes[idx],
                        bytes[idx + 1],
                        bytes[idx + 2],
                        bytes[idx + 3],
                    ]);
                    data.push(value);
                }
                cursor += data_length * 4;
                PackedEnum::U32(data)
            }
            _ => panic!("Invalid packed type"),
        };

        // Read mask and length
        let mask = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;

        let length = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;

        Chunk {
            data: PackedUints { data, mask, length },
            palette,
        }
    }
}

impl From<&[Block]> for Chunk {
    fn from(values: &[Block]) -> Self {
        let mut palette = Palette::new();
        palette.index(Block::Air);
        let values = values
            .iter()
            .map(|v| palette.index(v.clone()))
            .collect_vec();
        let data = PackedUints::from(values.as_slice());
        Chunk { data, palette }
    }
}

impl Chunk {
    pub fn new() -> Self {
        let mut palette = Palette::new();
        palette.index(Block::Air);
        Chunk {
            data: PackedUints::new(CHUNKP_S3),
            palette: palette,
        }
    }
}
