use crate::world::{BlockPos, Realm, VoxelWorld};
use crate::Block;
use bevy::{
    prelude::*,
    time::{Time, Timer},
};
use itertools::{iproduct, Itertools};
const FREE_FLY_Y_SPEED: f32 = 100.;
const ACC_MULT: f32 = 150.;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_stepped_block)
            .add_systems(Update, process_jumps)
            .add_systems(Update, process_freefly)
            .add_systems(Update, apply_acc_free_fly)
            .add_systems(Update, (apply_acc, apply_gravity, apply_velocity).chain());
    }
}

#[derive(Component)]
pub struct SteppingOn(pub Block);

#[derive(Component)]
pub struct Walking;

#[derive(Component)]
pub struct FreeFly;

#[derive(Component)]
pub struct Speed(pub f32);

#[derive(Component)]
pub struct Jumping {
    pub force: f32,
    pub cd: Timer,
    pub intent: bool,
}

#[derive(Component)]
pub struct Crouching(pub bool);

#[derive(Component)]
pub struct AABB(pub Vec3);

// both describe the speed and the direction the entity wants to go towards
#[derive(Component)]
pub struct Heading(pub Vec3);

// the actual speed vector of the entity, most of the time it is similar to Heading but not always!
#[derive(Component)]
pub struct Velocity(pub Vec3);

// the gravity that the entity will be subject to
#[derive(Component)]
pub struct Gravity(pub f32);

fn extent(v: f32, size: f32) -> Vec<i32> {
    if size > 0. {
        ((v.floor() as i32)..=((size + v).floor() as i32)).collect_vec()
    } else {
        (((size + v).floor() as i32)..=(v.floor() as i32))
            .rev()
            .collect_vec()
    }
}
fn blocks_perp_y(pos: Vec3, realm: Realm, aabb: &AABB) -> impl Iterator<Item = BlockPos> {
    // Apply scaling factor to the position
    let scaled_pos = pos * 8.0;
    iproduct!(
        extent(scaled_pos.x, aabb.0.x * 8.0),
        extent(scaled_pos.z, aabb.0.z * 8.0)
    )
    .map(move |(x, z)| BlockPos {
        x,
        y: scaled_pos.y.floor() as i32,
        z,
        realm: realm,
    })
}
fn blocks_perp_z(pos: Vec3, realm: Realm, aabb: &AABB) -> impl Iterator<Item = BlockPos> {
    // Apply scaling factor to the position
    let scaled_pos = pos * 8.0;
    iproduct!(
        extent(scaled_pos.x, aabb.0.x * 8.0),
        extent(scaled_pos.y, aabb.0.y * 8.0)
    )
    .map(move |(x, y)| BlockPos {
        x,
        y,
        z: scaled_pos.z.floor() as i32,
        realm: realm,
    })
}

fn blocks_perp_x(pos: Vec3, realm: Realm, aabb: &AABB) -> impl Iterator<Item = BlockPos> {
    // Apply scaling factor to the position
    let scaled_pos = pos * 8.0;
    iproduct!(
        extent(scaled_pos.y, aabb.0.y * 8.0),
        extent(scaled_pos.z, aabb.0.z * 8.0)
    )
    .map(move |(y, z)| BlockPos {
        x: scaled_pos.x.floor() as i32,
        y,
        z,
        realm: realm,
    })
}
fn update_stepped_block(
    blocks: Res<VoxelWorld>,
    mut query: Query<(&Transform, &Realm, &AABB, &mut SteppingOn)>,
) {
    for (transform, realm, aabb, mut stepping_on) in query.iter_mut() {
        let below = transform.translation + Vec3::new(0., -0.01, 0.);
        let mut closest_block = Block::Air;
        let mut min_dist = f32::INFINITY;
        for block_pos in blocks_perp_y(below, *realm, aabb) {
            let block = blocks.get_block(block_pos);
            if block.is_traversable() {
                continue;
            }
            let scaled_below = below * 8.0;
            let dist = (scaled_below.x - block_pos.x as f32).abs()
                - (scaled_below.y - block_pos.y as f32).abs();
            if dist < min_dist {
                min_dist = dist;
                closest_block = block;
            }
        }
        stepping_on.0 = closest_block;
    }
}
fn process_jumps(
    time: Res<Time>,
    mut query: Query<(&mut Jumping, &mut Velocity, &SteppingOn), With<Walking>>,
) {
    for (mut jumping, mut velocity, stepping_on) in query.iter_mut() {
        jumping.cd.tick(time.delta());
        if jumping.intent && jumping.cd.finished() && !stepping_on.0.is_traversable() {
            velocity.0.y += jumping.force;
            jumping.cd.reset();
        }
    }
}

fn process_freefly(mut query: Query<(&mut Velocity, &Jumping, &Crouching), With<FreeFly>>) {
    for (mut velocity, jumping, crouching) in query.iter_mut() {
        velocity.0.y = (jumping.intent as i32 - crouching.0 as i32) as f32 * FREE_FLY_Y_SPEED;
    }
}

fn apply_gravity(
    blocks: Res<VoxelWorld>,
    time: Res<Time>,
    mut query: Query<(&Transform, &Realm, &mut Velocity, &Gravity), With<Walking>>,
) {
    for (transform, realm, mut velocity, gravity) in query.iter_mut() {
        if !blocks.is_col_loaded(transform.translation, *realm) {
            continue;
        }
        velocity.0 += Vec3::new(0., -gravity.0 * time.delta_secs(), 0.);
    }
}

fn apply_acc(
    blocks: Res<VoxelWorld>,
    time: Res<Time>,
    mut query: Query<(&Heading, &mut Velocity, &Transform, &Realm, &SteppingOn), With<Walking>>,
) {
    for (heading, mut velocity, transform, realm, stepping_on) in query.iter_mut() {
        if !blocks.is_col_loaded(transform.translation, *realm) {
            continue;
        }
        // get the block the entity is standing on if the entity has an AABB
        let friction: f32 = stepping_on.0.friction();
        let slowing: f32 = stepping_on.0.slowing();
        // applying slowing
        let heading = heading.0 * slowing;
        // make velocity inch towards heading
        let mut diff = heading - velocity.0;
        if diff.y.is_nan() {
            diff.y = 0.;
        }
        let diff_len = diff.length();
        if diff_len == 0. {
            continue;
        }
        let c = (time.delta_secs() * friction * ACC_MULT / diff_len.max(1.)).min(1.);
        let acc: Vec3 = c * diff;
        velocity.0 += acc;
    }
}

fn apply_acc_free_fly(mut query: Query<(&Heading, &mut Velocity), With<FreeFly>>) {
    for (heading, mut velocity) in query.iter_mut() {
        velocity.0.x = heading.0.x;
        velocity.0.z = heading.0.z;
    }
}

fn apply_velocity(
    blocks: Res<VoxelWorld>,
    time: Res<Time>,
    mut query: Query<(&mut Velocity, &mut Transform, &Realm, &AABB)>,
) {
    for (mut velocity, mut transform, realm, aabb) in query.iter_mut() {
        if !blocks.is_col_loaded(transform.translation, *realm) {
            continue;
        }
        let applied_velocity = velocity.0 * time.delta_secs();

        // x axis
        let xpos = if applied_velocity.x > 0. {
            aabb.0.x + transform.translation.x
        } else {
            transform.translation.x
        };
        let mut stopped = false;
        // Scale the extent calculation
        for x in extent(xpos * 8.0, applied_velocity.x * 8.0)
            .into_iter()
            .skip(1)
        {
            let pos_x = Vec3 {
                x: x as f32 / 8.0, // Convert back to world space for other calculations
                y: transform.translation.y,
                z: transform.translation.z,
            };
            if blocks_perp_x(pos_x, *realm, aabb).any(|pos| !blocks.get_block(pos).is_traversable())
            {
                // there's a collision in this direction, stop at the block limit
                if applied_velocity.x > 0. {
                    transform.translation.x = (x as f32 / 8.0) - aabb.0.x - 0.001;
                } else {
                    transform.translation.x = (x as f32 / 8.0) + 1.001 / 8.0; // Scale the offset
                }
                velocity.0.x = 0.;
                stopped = true;
                break;
            }
        }
        if !stopped {
            transform.translation.x += applied_velocity.x;
        }

        // y axis (similar changes)
        let ypos: f32 = if applied_velocity.y > 0. {
            aabb.0.y + transform.translation.y
        } else {
            transform.translation.y
        };
        let mut stopped = false;
        for y in extent(ypos * 8.0, applied_velocity.y * 8.0)
            .into_iter()
            .skip(1)
        {
            let pos_y = Vec3 {
                x: transform.translation.x,
                y: y as f32 / 8.0,
                z: transform.translation.z,
            };
            if blocks_perp_y(pos_y, *realm, aabb).any(|pos| !blocks.get_block(pos).is_traversable())
            {
                if applied_velocity.y > 0. {
                    transform.translation.y = (y as f32 / 8.0) - aabb.0.y - 0.001;
                } else {
                    transform.translation.y = (y as f32 / 8.0) + 1.001 / 8.0;
                }
                velocity.0.y = 0.;
                stopped = true;
                break;
            }
        }
        if !stopped {
            transform.translation.y += applied_velocity.y;
        }

        // z axis (similar changes)
        let zpos: f32 = if applied_velocity.z > 0. {
            aabb.0.z + transform.translation.z
        } else {
            transform.translation.z
        };
        let mut stopped = false;
        for z in extent(zpos * 8.0, applied_velocity.z * 8.0)
            .into_iter()
            .skip(1)
        {
            let pos_z = Vec3 {
                x: transform.translation.x,
                y: transform.translation.y,
                z: z as f32 / 8.0,
            };
            if blocks_perp_z(pos_z, *realm, aabb).any(|pos| !blocks.get_block(pos).is_traversable())
            {
                if applied_velocity.z > 0. {
                    transform.translation.z = (z as f32 / 8.0) - aabb.0.z - 0.001;
                } else {
                    transform.translation.z = (z as f32 / 8.0) + 1.001 / 8.0;
                }
                velocity.0.z = 0.;
                stopped = true;
                break;
            }
        }
        if !stopped {
            transform.translation.z += applied_velocity.z;
        }
    }
}
