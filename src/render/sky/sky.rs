use crate::render::camera::CameraSpawn;
use bevy::{pbr::VolumetricLight, prelude::*};
use bevy_atmosphere::prelude::{AtmosphereCamera, AtmosphereModel, AtmospherePlugin, Nishita};
use std::f32::consts::PI;
const DAY_LENGTH_MINUTES: f32 = 0.2;
const C: f32 = DAY_LENGTH_MINUTES * 120. * PI;

// Timer for updating the daylight cycle (updating the atmosphere every frame is slow, so it's better to do incremental changes)
#[derive(Resource)]
struct CycleTimer(Timer);

#[derive(Component)]
pub struct Sun;

fn spawn_sun(mut commands: Commands, cam_query: Query<Entity, With<Camera3d>>) {
    let cam = cam_query.get_single().unwrap();
    commands.entity(cam).insert(AtmosphereCamera::default());
    commands.spawn((
        Sun,
        VolumetricLight,
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10000.,
            shadows_enabled: true,
            ..Default::default()
        },
    ));
}

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 800.0,
        })
        .insert_resource(AtmosphereModel::new(Nishita { ..default() }))
        .insert_resource(CycleTimer(Timer::new(
            // Update our atmosphere every 500ms
            bevy::utils::Duration::from_millis(500),
            TimerMode::Repeating,
        )))
        .add_plugins(AtmospherePlugin)
        .add_systems(Startup, spawn_sun.after(CameraSpawn));
    }
}
