use crate::{
    asset_processing::from_filename,
    block::{Face, FaceSpecifier},
    Block,
};
use bevy::{asset::LoadedFolder, prelude::*};
use std::ffi::OsStr;

const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

pub struct TextureLoadPlugin;

impl Plugin for TextureLoadPlugin {
    fn build(&self, app: &mut App) {
        app;
    }
}
