#![feature(impl_trait_in_assoc_type)]
#![feature(portable_simd)]
#![feature(const_trait_impl)]
#![feature(f16)]

use wasm_bindgen::prelude::*;

mod agents;
mod block;
mod controls;
mod gen;
mod interactions;
mod items;
mod render;
mod scenes;
mod setup;
mod sounds;
mod ui;
mod utils;
mod world;

use setup::*;

#[wasm_bindgen]
pub fn setup() {
    println!("setup");
    create_app();
}
