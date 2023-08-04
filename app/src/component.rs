use crate::animation::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::sprite::Mesh2dHandle;
use bevy::{prelude::*, sprite::Anchor};

pub use crate::collision::Hitbox;

#[derive(Component, Debug)]
pub struct Player {
    pub id: usize,
}

#[derive(Component, Debug)]
pub struct Bow;

#[derive(Component, Reflect, Default, Debug)]
pub struct Velocity(pub Vec2);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Component, Reflect, Default, Debug)]
pub struct Lifetime(pub usize);

#[derive(Component, Reflect, Default, Debug)]
pub struct InputAngle(pub u8);

#[derive(Component, Reflect, Default, Debug)]
pub struct CanShoot {
    pub value: bool,
    pub since_last: usize,
}

#[derive(Component, Debug)]
pub struct Bullet {
    pub dir: Vec2,
    pub vel: f32,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Component)]
pub struct FollowPlayer;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct MinimapCamera;

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
            transform: TransformBundle::default(),
        }
    }
}

#[derive(Component, Debug)]
pub struct AnimationTimer(pub Timer);

#[derive(Component)]
pub struct Tilemap;

#[derive(Bundle)]
pub struct TilemapBundle {
    tilemap: Tilemap,
    mesh: MaterialMesh2dBundle<ColorMaterial>,
}

impl TilemapBundle {
    pub fn new(mesh: Handle<Mesh>, material: Handle<ColorMaterial>) -> Self {
        TilemapBundle {
            tilemap: Tilemap,
            mesh: MaterialMesh2dBundle {
                mesh: Mesh2dHandle(mesh),
                material,
                ..default()
            },
        }
    }
}

#[derive(Bundle)]
pub struct BulletBundle {
    bullet: Bullet,
    sprite: SpriteBundle,
    lifetime: Lifetime,
    hitbox: Hitbox,
}

impl BulletBundle {
    pub fn new(dir: Vec2, vel: f32, lifetime: usize, texture: Handle<Image>) -> Self {
        Self {
            bullet: Bullet { dir, vel },
            sprite: SpriteBundle {
                texture,
                ..default()
            },
            lifetime: Lifetime(lifetime),
            hitbox: Hitbox::Circle {
                pos: Vec2::splat(3.),
                radius: 2.5,
            },
        }
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    sprite: SpriteSheetBundle,
    velocity: Velocity,
    facing: Facing,
    timer: AnimationTimer,
    indices: AnimationIndices,
    can_shoot: CanShoot,
    hitbox: Hitbox,
    input_angle: InputAngle,
}

impl PlayerBundle {
    pub fn new(id: usize, atlas: Handle<TextureAtlas>) -> Self {
        Self {
            player: Player { id },
            sprite: SpriteSheetBundle {
                texture_atlas: atlas,
                sprite: TextureAtlasSprite::new(0),
                ..default()
            },
            velocity: Velocity(Vec2::ZERO),
            facing: Facing::Down,
            timer: AnimationTimer(Timer::from_seconds(0.125, TimerMode::Repeating)),
            indices: PLAYER_STAND_DOWN,
            can_shoot: CanShoot {
                value: true,
                since_last: 999,
            },
            hitbox: Hitbox::Rect {
                pos: Vec2::ZERO,
                half_size: Vec2::splat(4.),
            },
            input_angle: InputAngle(0),
        }
    }
}

#[derive(Bundle)]
pub struct BowBundle {
    bow: Bow,
    sprite: SpriteSheetBundle,
    timer: AnimationTimer,
    indices: AnimationIndices,
}

impl BowBundle {
    pub fn new(atlas: Handle<TextureAtlas>) -> Self {
        Self {
            bow: Bow,
            sprite: SpriteSheetBundle {
                texture_atlas: atlas,
                // z is -1 to indicate "below player"
                transform: Transform::from_xyz(0., 0., -1.),
                sprite: TextureAtlasSprite {
                    index: 0,
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                ..default()
            },
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Once)),
            indices: BOW_DRAW,
        }
    }
}
