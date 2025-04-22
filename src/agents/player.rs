use super::{block_action::BlockActionPlugin, key_binds::KeyBinds};
use crate::world::{BlockPos, BlockRayCastHit, Realm, VoxelWorld};
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
            .insert_resource(PlayerUnlockTimer::default())
            .insert_resource(StepUpSettings {
                max_step_height: 1.0,
                smoothing_factor: 0.15,
                gravity_multiplier: 2.0,
            })
            .add_plugins(BlockActionPlugin)
            .add_plugins(InputManagerPlugin::<Dir>::default())
            .add_plugins(InputManagerPlugin::<Action>::default())
            .add_plugins(InputManagerPlugin::<DevCommand>::default())
            .add_systems(
                Startup,
                (spawn_player, apply_deferred).chain().in_set(PlayerSpawn),
            )
            .add_systems(
                Update,
                check_unlock_player.run_if(resource_exists::<PlayerUnlockTimer>),
            )
            .add_systems(
                Update,
                (smooth_player_step, move_player).run_if(player_phsysics_ready),
            );
    }
}
#[derive(Resource)]
pub struct PlayerUnlockTimer {
    timer: Timer,
}

impl Default for PlayerUnlockTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        }
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
#[derive(Resource)]
pub struct StepUpSettings {
    pub max_step_height: f32,
    pub smoothing_factor: f32,
    pub gravity_multiplier: f32,
}

pub fn check_unlock_player(
    mut commands: Commands,
    mut timer: ResMut<PlayerUnlockTimer>,
    time: Res<Time>,
    player_query: Query<Entity, With<PlayerControlled>>,
) {
    timer.timer.tick(time.delta());
    if timer.timer.finished() {
        info!("World loaded, unlocking player");
        let entity = player_query.single();
        commands.entity(entity).insert((
            RigidBody::Dynamic,
            Collider::sphere(2.0),
            LinearVelocity(Vec3::new(0., 0., 0.)),
            LockedAxes::new()
                .lock_rotation_y()
                .lock_rotation_x()
                .lock_rotation_z(),
        ));
        commands.remove_resource::<PlayerUnlockTimer>();
    }
}

pub fn spawn_player(
    mut commands: Commands,
    key_binds: Res<KeyBinds>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let realm = Realm::Overworld;

    let rd = RenderDistance(4);
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
            rd,
            TargetBlock(None),
            PlayerControlled,
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
    mut player_query: Query<(&mut LinearVelocity, &ActionState<Dir>), With<PlayerControlled>>,
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
        velocity.0 = movement * WALK_SPEED * delta_secs;
    } else {
        velocity.0 = Vec3::ZERO;
    }
}

pub fn smooth_player_step(
    mut player_query: Query<(&Transform, &mut LinearVelocity), With<PlayerControlled>>,
    world: Res<VoxelWorld>,
    step_settings: Res<StepUpSettings>,
    time: Res<Time>,
) {
    let (transform, mut velocity) = player_query.single_mut();
    let player_pos = transform.translation;

    // Check if player is on ground or close to it
    let below_pos = player_pos + Vec3::new(0.0, -1.1, 0.0);
    let block_below = world.get_block_safe(BlockPos::from((below_pos, Realm::Overworld)));

    // Check for a step in front of the player in the direction of movement
    let movement_dir = Vec3::new(velocity.0.x, 0.0, velocity.0.z).normalize_or_zero();
    if movement_dir.length_squared() > 0.01 {
        let front_pos = player_pos + movement_dir * 1.1;
        let front_block = world.get_block_safe(BlockPos::from((front_pos, Realm::Overworld)));

        // Check for step up (solid block in front but air above it)
        if front_block != Block::Air {
            for step_height in 1..=(step_settings.max_step_height as i32) {
                let step_check_pos = front_pos + Vec3::new(0.0, step_height as f32, 0.0);
                let above_step =
                    world.get_block_safe(BlockPos::from((step_check_pos, Realm::Overworld)));

                if above_step == Block::Air {
                    // Apply upward velocity to step up
                    velocity.0.y = velocity
                        .0
                        .y
                        .max(step_settings.smoothing_factor * time.delta_secs() * 600.0);
                    break;
                }
            }
        }
    }

    // Apply stronger downward force when falling
    if block_below == Block::Air && velocity.0.y < 0.0 {
        velocity.0.y *= step_settings.gravity_multiplier;
    }
}
fn player_phsysics_ready(
    player_query: Query<(&Transform, &LinearVelocity), With<PlayerControlled>>,
) -> bool {
    !player_query.is_empty()
}
