use super::{block_action::BlockActionPlugin, key_binds::KeyBinds};
use crate::world::{BlockRayCastHit, Realm};
use crate::{world::RenderDistance, Block};
use avian3d::prelude::{AngularVelocity, Collider, LinearVelocity, LockedAxes, RigidBody};
use bevy::{math::Vec3, prelude::*};
use leafwing_input_manager::prelude::*;
use std::time::Duration;

const WALK_SPEED: f32 = 200.;
const FREE_FLY_X_SPEED: f32 = 150.;
const SPAWN: Vec3 = Vec3 {
    x: 500.,
    y: 8.,
    z: 500.,
};

pub struct PlayerPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, SystemSet)]
pub struct PlayerSpawn;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(KeyBinds::default())
            .add_plugins(BlockActionPlugin)
            .add_plugins(InputManagerPlugin::<Dir>::default())
            .add_plugins(InputManagerPlugin::<Action>::default())
            .add_plugins(InputManagerPlugin::<DevCommand>::default())
            .add_systems(
                Startup,
                (spawn_player, apply_deferred).chain().in_set(PlayerSpawn),
            )
            .add_systems(Update, move_player);
    }
}

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct TargetBlock(pub Option<BlockRayCastHit>);

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum Dir {
    Front,
    Back,
    Up,
    Left,
    Down,
    Right,
}

impl From<Dir> for Vec3 {
    fn from(dir: Dir) -> Self {
        match dir {
            Dir::Front => Vec3::new(0., 0., 1.),
            Dir::Back => Vec3::new(0., 0., -1.),
            Dir::Up => Vec3::new(0., 1., 0.),
            Dir::Down => Vec3::new(0., -1., 0.),
            Dir::Right => Vec3::new(1., 0., 0.),
            Dir::Left => Vec3::new(-1., 0., 0.),
        }
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Debug, Hash, Reflect)]
pub enum Action {
    Hit,
    Modify,
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Debug, Hash, Reflect)]
pub enum DevCommand {
    ToggleFly,
}

pub fn spawn_player(
    mut commands: Commands,
    key_binds: Res<KeyBinds>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let realm = Realm::Overworld;
    // Render distance nerfed from 64 to 32 (4km to 2km) while we don't have instancing
    let rd = RenderDistance(3);

    let player_model = commands
        .spawn((
            Transform::from_xyz(0., 0.5, 0.),
            Mesh3d(meshes.add(Capsule3d::new(0.5, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 0, 0))),
        ))
        .id();
    commands
        .spawn((
            Transform {
                translation: SPAWN,
                ..default()
            },
            Visibility::default(),
            realm,
            Mesh3d(meshes.add(Capsule3d::new(0.5, 1.0))),
            rd,
            TargetBlock(None),
            PlayerControlled,
        ))
        .insert((
            RigidBody::Dynamic,
            Collider::cylinder(0.5, 1.),
            LinearVelocity(Vec3::new(0., 0., 0.)),
            LockedAxes::new()
                .lock_rotation_y()
                .lock_rotation_x()
                .lock_rotation_z(),
        ))
        .insert(SpatialListener::new(0.3))
        .insert(InputManagerBundle::<Dir> {
            action_state: ActionState::default(),
            input_map: InputMap::new([
                (Dir::Front, key_binds.forward),
                (Dir::Left, key_binds.left),
                (Dir::Back, key_binds.backward),
                (Dir::Right, key_binds.right),
            ]),
        })
        .insert(InputManagerBundle::<Action> {
            action_state: ActionState::default(),
            input_map: InputMap::new([
                (Action::Hit, key_binds.hit),
                (Action::Modify, key_binds.modify),
            ]),
        })
        .insert(InputManagerBundle::<DevCommand> {
            action_state: ActionState::default(),
            input_map: InputMap::new([(DevCommand::ToggleFly, key_binds.toggle_fly)]),
        })
        .add_child(player_model);
}

pub fn move_player(
    mut player_query: Query<(&mut LinearVelocity, &ActionState<Dir>)>,
    cam_query: Query<&Transform, With<Camera>>,
    time: Res<Time>,
) {
    let cam_transform = if let Ok(ct) = cam_query.get_single() {
        *ct
    } else {
        Transform::default()
    };
    let (mut velocity, action_state) = player_query.single_mut();

    let delta_secs = time.delta_secs();
    let mut movement = Vec3::default();
    for action in action_state.get_pressed() {
        movement += Vec3::from(action);
    }
    if movement.length_squared() > 0. {
        movement = movement.normalize();
        movement = Vec3::Y.cross(*cam_transform.right()) * movement.z
            + cam_transform.right() * movement.x
            + movement.y * Vec3::Y;
        movement.y = 0.;
        velocity.0 = movement * WALK_SPEED * delta_secs;
    }
}
