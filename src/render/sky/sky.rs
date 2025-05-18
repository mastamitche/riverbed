use crate::render::camera::CameraSpawn;
use bevy::{pbr::light_consts::lux::OVERCAST_DAY, prelude::*};
use std::{f32::consts::PI, time::Duration};
const DAY_LENGTH_MINUTES: f32 = 0.2;
const C: f32 = DAY_LENGTH_MINUTES * 120. * PI;

// Timer for updating the daylight cycle (updating the atmosphere every frame is slow, so it's better to do incremental changes)
#[derive(Resource)]
struct CycleTimer(Timer);

#[derive(Component)]
pub struct Sun;

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        Sun,
        DirectionalLight {
            color: Color::srgb_u8(201, 226, 255),
            illuminance: OVERCAST_DAY,
            // Cant do shadows for now
            // Get errors with custom pipeline
            shadows_enabled: true,
            ..Default::default()
        },
    ));
}

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbientLight {
            color: Color::srgb_u8(201, 226, 255),
            brightness: 300.,
            affects_lightmapped_meshes: false,
        })
        .insert_resource(CycleTimer(Timer::new(
            // Update our atmosphere every 500ms
            Duration::from_millis(500),
            TimerMode::Repeating,
        )))
        .add_systems(Startup, spawn_sun.after(CameraSpawn));
    }
}
