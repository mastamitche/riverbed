#![feature(impl_trait_in_assoc_type)]
#![feature(portable_simd)]
#![feature(const_trait_impl)]
#![feature(f16)]

mod agents;
mod block;
mod gen;
mod items;
mod render;
mod setup;
mod sounds;
mod ui;
mod world;

use setup::*;
use wasm_bindgen::prelude::wasm_bindgen;
pub fn main() {
    create_app();
}
#[wasm_bindgen]
pub fn setup() {
    main();
}
