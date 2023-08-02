use crate::component::Hitbox;
use bevy::prelude::*;
use sepax2d::prelude::*;

#[derive(Clone, Copy, Debug)]
pub enum HitboxShape {
    Rect { pos: Vec2, half_size: Vec2 },
    Circle { pos: Vec2, radius: f32 },
}

impl HitboxShape {
    pub fn with_transform(&self, transform: &Transform) -> HitboxShape {
        match self {
            HitboxShape::Rect { pos, half_size } => {
                let pos = transform.transform_point(pos.extend(0.)).truncate();
                let half_size = Vec2 {
                    x: transform.scale.x * half_size.x,
                    y: transform.scale.y * half_size.y,
                };
                HitboxShape::Rect { pos, half_size }
            }
            HitboxShape::Circle { pos, radius } => {
                let pos = transform.transform_point(pos.extend(0.)).truncate();
                let scale = transform.scale.max_element();
                let radius = radius * scale;
                HitboxShape::Circle { pos, radius }
            }
        }
    }

    pub fn into_sepax(&self) -> Box<dyn Shape> {
        match self {
            HitboxShape::Rect { pos, half_size } => {
                let top_left = *pos - *half_size;
                Box::new(AABB {
                    position: (top_left.x, top_left.y),
                    width: half_size.x * 2.,
                    height: half_size.y * 2.,
                })
            }
            HitboxShape::Circle { pos, radius } => Box::new(Circle {
                position: (pos.x, pos.y),
                radius: *radius,
            }),
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
        .add_systems(Update, collision_debug_draw);
    }
}

fn collision_debug_draw(mut gizmos: Gizmos, query: Query<(&Hitbox, &Transform)>) {
    for (hitbox, transform) in &query {
        match hitbox.shape.with_transform(transform) {
            HitboxShape::Circle { pos, radius } => {
                gizmos.circle_2d(pos, radius, Color::RED);
            }
            HitboxShape::Rect { pos, half_size } => {
                gizmos.rect_2d(pos, 0., half_size * 2., Color::RED);
            }
        }
    }
}

pub fn hitbox_intersects(a: (&HitboxShape, &Transform), b: (&HitboxShape, &Transform)) -> bool {
    let shape_a = a.0.with_transform(a.1).into_sepax();
    let shape_b = b.0.with_transform(b.1).into_sepax();
    sat_overlap(shape_a.as_ref(), shape_b.as_ref())
}
