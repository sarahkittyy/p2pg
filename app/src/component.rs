use crate::{
    animation::*,
    collision::{Hitbox, WallSensors},
};
use bevy::{prelude::*, sprite::Anchor};

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

#[derive(Component)]
pub struct Bullet;

#[derive(Component, Clone, Copy, Reflect, Debug, Default, PartialEq, Eq)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Component, Debug, Reflect)]
pub struct AnimationTimer(pub Timer);

#[derive(Component)]
pub struct FollowPlayer;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct MinimapCamera;

#[derive(Component)]
pub struct Tilemap;

#[derive(Component, Default, Debug, Reflect)]
pub struct WallContactState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Component, Default, Debug, Reflect)]
pub struct Spawnpoints(pub Vec<Vec2>);

#[derive(Bundle)]
pub struct BulletBundle {
    bullet: Bullet,
    velocity: Velocity,
    sprite: SpriteBundle,
    lifetime: Lifetime,
    hitbox: Hitbox,
}

impl BulletBundle {
    pub fn new(dir: Vec2, vel: f32, lifetime: usize, texture: Handle<Image>) -> Self {
        Self {
            bullet: Bullet,
            velocity: Velocity(dir.normalize_or_zero() * vel),
            sprite: SpriteBundle {
                texture,
                ..default()
            },
            lifetime: Lifetime(lifetime),
            hitbox: Hitbox::Circle {
                offset: Vec2::splat(3.),
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
    wall_sensors: WallSensors,
    wall_contact_state: WallContactState,
    input_angle: InputAngle,
}

impl PlayerBundle {
    pub fn new(id: usize, atlas: Handle<TextureAtlas>) -> Self {
        const SIZE: f32 = 4.1;
        const E: f32 = 0.05;
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
                offset: Vec2::ZERO,
                half_size: Vec2::splat(SIZE),
            },
            wall_sensors: WallSensors {
                up: Hitbox::Rect {
                    offset: Vec2::Y * SIZE,
                    half_size: Vec2::new(SIZE - E, E),
                },
                down: Hitbox::Rect {
                    offset: Vec2::NEG_Y * SIZE,
                    half_size: Vec2::new(SIZE - E, E),
                },
                left: Hitbox::Rect {
                    offset: Vec2::NEG_X * SIZE,
                    half_size: Vec2::new(E, SIZE - E),
                },
                right: Hitbox::Rect {
                    offset: Vec2::X * SIZE,
                    half_size: Vec2::new(E, SIZE - E),
                },
            },
            wall_contact_state: WallContactState::default(),
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
