use anyhow::{bail, Result};
use itertools::Itertools;
use std::str::FromStr;

use super::ClosestTrait;

trait PointDistSq {
    fn dist(&self, other: &Self) -> f32;
}

impl<const D: usize> PointDistSq for [f32; D] {
    fn dist(&self, other: &Self) -> f32 {
        let mut res = 0.;
        for i in 0..D {
            res += (self[i] - other[i]).powi(2);
        }
        res
    }
}

impl<const D: usize, E: Clone> ClosestTrait<D, E> for Vec<([f32; D], E)> {
    fn closest(&self, point: [f32; D]) -> (&E, f32) {
        let mut candidates = self
            .iter()
            .map(|(points, value)| (value, points.dist(&point)));
        let mut closest1 = candidates.next().unwrap();
        let Some(mut closest2) = candidates.next() else {
            return closest1;
        };
        if closest2.1 < closest1.1 {
            (closest1, closest2) = (closest2, closest1);
        }
        for (v, dist) in candidates {
            if dist < closest1.1 {
                closest2 = closest1;
                closest1 = (v, dist);
            } else if dist < closest2.1 {
                closest2 = (v, dist);
            }
        }
        (closest1.0, 1. - 2. * closest1.1 / (closest1.1 + closest2.1))
    }

    fn values(&self) -> Vec<&E> {
        self.iter().map(|(_, value)| value).collect_vec()
    }
}

pub fn from_csv<const D: usize, E: FromStr>(path: &str) -> Result<Vec<([f32; D], E)>> {
    let mut res = Vec::new();
    let mut reader = csv::Reader::from_path(path)?;
    for record in reader.records() {
        let record = record?;
        let Ok(elem) = E::from_str(&record[0]) else {
            bail!("Failed to deserialize value '{}'", &record[0]);
        };
        let intervals: [f32; D] =
            core::array::from_fn(|i| record[i + 1].trim().parse::<f32>().unwrap());
        res.push((intervals, elem));
    }
    Ok(res)
}
