mod earth_gen;
pub mod terrain_gen;
use std::ops::Range;

use crate::block::Block;

type Soils = Vec<([Range<f32>; 2], Block)>;
