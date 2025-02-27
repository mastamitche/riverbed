use crate::{
    asset_processing::from_filename,
    block::{Face, FaceSpecifier},
    Block,
};
use bevy::{
    asset::LoadedFolder, color::palettes::css, prelude::*,
    render::texture::TRANSPARENT_IMAGE_HANDLE,
};
use itertools::Itertools;
use std::collections::HashMap;
pub const SLOT_SIZE_PERCENT: f32 = 4.;

pub struct UiTexMapPlugin;

impl Plugin for UiTexMapPlugin {
    fn build(&self, app: &mut App) {
        app;
    }
}
