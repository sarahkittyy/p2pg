use bevy::prelude::*;
use sepax2d::prelude::*;

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

pub struct DebugHitboxPlugin;
impl Plugin for DebugHitboxPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GizmoConfig {
            depth_bias: -1.,
            ..default()
        })
        .register_type::<Hitbox>()
        .add_systems(Update, collision_debug_draw);
    }
}

fn collision_debug_draw(mut gizmos: Gizmos, q_hitbox: Query<(&Hitbox, &GlobalTransform)>) {
    for (hitbox, transform) in &q_hitbox {
        match hitbox.with_transform(&transform.compute_transform()) {
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
