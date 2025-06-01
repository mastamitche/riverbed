mod closest;
mod counter;
pub mod points;
pub mod ranges;
mod utils;
use crate::utils::math::counter::Counter;
pub use closest::*;
use std::fmt::Debug;

pub fn print_coverage<const D: usize, E: Clone + PartialEq + Debug>(
    imap: impl ClosestTrait<D, E>,
    step: f32,
) {
    let mut coverage = imap.coverage(step);
    // add missing values
    imap.values().into_iter().for_each(|value| {
        if !coverage.iter().any(|(v, _)| *v == value) {
            coverage.push((value, 0.))
        }
    });
    // .sum() refuses to work here for some reason
    let unassigned = 1f32
        - coverage
            .iter()
            .map(|(_, count)| *count)
            .fold(0., |a, b| a + b);
    coverage.ordered();
    for (value, count) in coverage {
        println!("{:?}: {:.1}%", value, count * 100.);
    }
    println!("---");
    println!("Empty: {:.1}%", unassigned * 100.);
}
