use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

#[derive(
    Debug,
    Display,
    PartialEq,
    EnumIter,
    EnumString,
    Eq,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Hash,
    FromPrimitive,
    ToPrimitive,
)]
pub enum BlockFamily {
    Default,
    Empty,
    Wood,
    Foliage,
    Stone,
    Ore,
    Utility,
}

#[derive(
    Debug,
    Display,
    PartialEq,
    EnumIter,
    EnumString,
    Eq,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Hash,
    FromPrimitive,
    ToPrimitive,
)]
pub enum Block {
    Air,
    OakLog,
    SpruceLog,
    BirchLog,
    Stone,
    IronOre,
    GoldOre,
    Furnace,
    FurnaceOn,
    DepletedIronOre,
    OakLeaves,
    SpruceLeaves,
    BirchLeaves,
}

impl Block {
    pub fn families(&self) -> Vec<BlockFamily> {
        match self {
            Block::Air => vec![BlockFamily::Empty],
            Block::OakLog | Block::SpruceLog | Block::BirchLog => vec![BlockFamily::Wood],
            Block::OakLeaves | Block::SpruceLeaves | Block::BirchLeaves => {
                vec![BlockFamily::Foliage]
            }
            Block::Stone => vec![BlockFamily::Stone],
            Block::IronOre | Block::GoldOre | Block::DepletedIronOre => vec![BlockFamily::Ore],
            Block::Furnace | Block::FurnaceOn => vec![BlockFamily::Utility],
        }
    }

    pub fn is_foliage(&self) -> bool {
        self.families().contains(&BlockFamily::Foliage)
    }

    pub fn depleted(&self) -> Block {
        match self {
            Block::IronOre => Block::DepletedIronOre,
            _ => *self,
        }
    }

    pub fn renewed(&self) -> Block {
        match self {
            Block::DepletedIronOre => Block::IronOre,
            _ => *self,
        }
    }

    pub fn renewal_minutes(&self) -> Option<u32> {
        match self {
            Block::DepletedIronOre => Some(10),
            _ => None,
        }
    }

    pub fn on(&self) -> Block {
        match self {
            Block::Furnace => Block::FurnaceOn,
            _ => *self,
        }
    }

    pub fn off(&self) -> Block {
        match self {
            Block::FurnaceOn => Block::Furnace,
            _ => *self,
        }
    }

    pub fn furnace_temp(&self) -> Option<u32> {
        match self {
            Block::Furnace | Block::FurnaceOn => Some(1200),
            _ => None,
        }
    }

    pub fn to_id(&self) -> u32 {
        ToPrimitive::to_u32(self).unwrap_or(0)
    }

    pub fn from_id(id: u32) -> Self {
        FromPrimitive::from_u32(id).unwrap_or(Block::Air)
    }

    pub fn is_targetable(&self) -> bool {
        let untargetable_families = [BlockFamily::Utility, BlockFamily::Empty];
        !untargetable_families
            .iter()
            .any(|family| self.families().contains(family))
    }
    pub fn is_opaque(&self) -> bool {
        let not_opaque = [BlockFamily::Utility, BlockFamily::Empty];
        !not_opaque
            .iter()
            .any(|family| self.families().contains(family))
    }
}
