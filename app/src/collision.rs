use bevy::{
    audio::{PlaybackMode, Volume, VolumeLevel},
    prelude::*,
};
use sepax2d::prelude::*;

use crate::{
    component::{Bullet, Health, LastDamagedBy, Player, Velocity},
    DebugState,
};

#[derive(Clone, Copy, Debug, Component, Reflect)]
pub enum Hitbox {
    Rect { offset: Vec2, half_size: Vec2 },
    Circle { offset: Vec2, radius: f32 },
}

impl Hitbox {
    pub fn with_transform(&self, transform: &Transform) -> Hitbox {
        match self {
            Hitbox::Rect { offset, half_size } => {
                let offset = transform.transform_point(offset.extend(0.)).truncate();
                let half_size = Vec2 {
                    x: transform.scale.x * half_size.x,
                    y: transform.scale.y * half_size.y,
                };
                Hitbox::Rect { offset, half_size }
            }
            Hitbox::Circle { offset, radius } => {
                let offset = transform.transform_point(offset.extend(0.)).truncate();
                let scale = transform.scale.max_element();
                let radius = radius * scale;
                Hitbox::Circle { offset, radius }
            }
        }
    }

    pub fn into_sepax(&self) -> Box<dyn Shape> {
        match self {
            Hitbox::Rect { offset, half_size } => {
                let top_left = *offset - *half_size;
                Box::new(AABB {
                    position: (top_left.x, top_left.y),
                    width: half_size.x * 2.,
                    height: half_size.y * 2.,
                })
            }
            Hitbox::Circle { offset, radius } => Box::new(Circle {
                position: (offset.x, offset.y),
                radius: *radius,
            }),
        }
    }
}

#[derive(Component)]
pub struct WallSensors {
    pub up: Hitbox,
    pub down: Hitbox,
    pub left: Hitbox,
    pub right: Hitbox,
}

#[derive(Component)]
pub struct RigidBody;

#[derive(Bundle)]
pub struct RigidBodyBundle {
    marker: RigidBody,
    hitbox: Hitbox,
    transform: TransformBundle,
}

impl RigidBodyBundle {
    pub fn new(hitbox: Hitbox) -> Self {
        Self {
            marker: RigidBody,
            hitbox,
            transform: TransformBundle::IDENTITY,
        }
    }
}

/// collisions between bullets and players
pub fn bullet_player_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_bullet: Query<(Entity, &Bullet, &Hitbox, &Transform), Without<Player>>,
    mut q_player: Query<
        (Entity, &Hitbox, &Transform, &mut Health),
        (With<Player>, Without<Bullet>),
    >,
) {
    for (p_entity, p_hitbox, p_transform, mut p_health) in &mut q_player {
        for (b_entity, bullet, b_hitbox, b_transform) in &q_bullet {
            if hitbox_intersects((p_hitbox, p_transform), (b_hitbox, b_transform)) {
                commands.spawn(AudioBundle {
                    source: asset_server.load("sfx/Damage_1.wav"),
                    settings: PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: Volume::Relative(VolumeLevel::new(0.3)),
                        speed: 1.,
                        ..default()
                    },
                });
                p_health.0 -= 1;
                commands
                    .entity(p_entity)
                    .insert(LastDamagedBy { id: bullet.shot_by });
                commands.entity(b_entity).despawn();
            }
        }
    }
}

/// reflect bullets that interact with solid terrain
pub fn _bullet_terrain_system(
    mut _commands: Commands,
    mut q_bullet: Query<
        (Entity, &mut Velocity, &Hitbox, &Transform),
        (With<Bullet>, Without<RigidBody>),
    >,
    q_rigidbody: Query<(&Hitbox, &Transform), (With<RigidBody>, Without<Bullet>)>,
) {
    for (_b_entity, mut b_vel, b_hitbox, b_transform) in &mut q_bullet {
        for (r_hitbox, r_transform) in &q_rigidbody {
            // the arrow should bounce in this direction
            let resolution = hitbox_collision((b_hitbox, b_transform), (r_hitbox, r_transform));
            if resolution.x.abs() > 0. {
                b_vel.0.x = b_vel.0.x.abs() * resolution.x.signum();
            }
            if resolution.y.abs() > 0. {
                b_vel.0.y = b_vel.0.y.abs() * resolution.y.signum();
            }
        }
    }
}

/// stop players from running into solid terrain
pub fn player_terrain_system(
    mut q_player: Query<(&Hitbox, &mut Transform), (With<Player>, Without<RigidBody>)>,
    q_rigidbody: Query<(&Hitbox, &Transform), (With<RigidBody>, Without<Player>)>,
) {
    for (p_hitbox, mut p_transform) in &mut q_player {
        for (r_hitbox, r_transform) in &q_rigidbody {
            let resolution = hitbox_collision((p_hitbox, &p_transform), (r_hitbox, &r_transform));
            p_transform.translation.x += resolution.x;
            p_transform.translation.y += resolution.y;
        }
    }
}

pub struct DebugHitboxPlugin;
impl Plugin for DebugHitboxPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GizmoConfig {
            depth_bias: -1.,
            ..default()
        })
        .register_type::<Hitbox>()
        .add_systems(
            Update,
            collision_debug_draw.run_if(in_state(DebugState::On)),
        );
    }
}

fn collision_debug_draw(mut gizmos: Gizmos, q_hitbox: Query<(&Hitbox, &Transform)>) {
    for (hitbox, transform) in &q_hitbox {
        match hitbox.with_transform(transform) {
            Hitbox::Circle { offset, radius } => {
                gizmos.circle_2d(offset, radius, Color::RED);
            }
            Hitbox::Rect { offset, half_size } => {
                gizmos.rect_2d(offset, 0., half_size * 2., Color::RED);
            }
        }
    }
}

/// returns the translation required to shift the dynamic body to not overlap with the rigid body
pub fn hitbox_collision(dynamic: (&Hitbox, &Transform), rigid: (&Hitbox, &Transform)) -> Vec2 {
    let shape_dynamic = dynamic.0.with_transform(dynamic.1).into_sepax();
    let shape_rigid = rigid.0.with_transform(rigid.1).into_sepax();
    Vec2::from(sat_collision(shape_rigid.as_ref(), shape_dynamic.as_ref()))
}

/// check if two transformed hitboxes intersect
pub fn hitbox_intersects(a: (&Hitbox, &Transform), b: (&Hitbox, &Transform)) -> bool {
    let shape_a = a.0.with_transform(a.1).into_sepax();
    let shape_b = b.0.with_transform(b.1).into_sepax();
    sat_overlap(shape_a.as_ref(), shape_b.as_ref())
}
