mod debug_gen;
mod earth_gen;
pub mod terrain_gen;

use crate::Block;
use std::ops::Range;

type Soils = Vec<([Range<f32>; 2], Block)>;
