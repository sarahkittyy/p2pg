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

#[derive(Component, Clone, Copy, Reflect, Default, Debug)]
pub struct Velocity(pub Vec2);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Component, Clone, Copy, Reflect, Default, Debug)]
pub struct Lifetime(pub usize);

#[derive(Component, Clone, Copy, Reflect, Default, Debug)]
pub struct InputAngle(pub u8);

#[derive(Component, Clone, Copy, Reflect, Default, Debug)]
pub struct CanShoot {
    pub value: bool,
    pub since_last: usize,
}

#[derive(Component)]
pub struct Bullet {
    pub shot_by: usize,
}

#[derive(Component, Clone, Copy, Default, Debug, Reflect)]
pub struct Health(pub i32);

#[derive(Component, Clone, Copy, Default, Debug, Reflect)]
pub struct LastDamagedBy {
    pub id: usize,
}

#[derive(Component)]
pub struct FollowPlayer;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct MinimapCamera;

#[derive(Component, Clone, Copy, Reflect, Debug, Default)]
pub struct Points(pub u32);

#[derive(Component, Clone, Copy, Default, Debug, Reflect)]
pub struct WallContactState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Component, Default, Debug, Reflect)]
pub struct Spawnpoint;

#[derive(Bundle)]
pub struct BulletBundle {
    bullet: Bullet,
    velocity: Velocity,
    sprite: SpriteBundle,
    lifetime: Lifetime,
    hitbox: Hitbox,
}

impl BulletBundle {
    pub fn new(
        shot_by: usize,
        dir: Vec2,
        vel: f32,
        lifetime: usize,
        texture: Handle<Image>,
    ) -> Self {
        Self {
            bullet: Bullet { shot_by },
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

// a player bundle consisting only of components important to reset on death
// so that it can be inserted into the player entity to reload them
#[derive(Bundle)]
pub struct BasePlayerBundle {
    velocity: Velocity,
    can_shoot: CanShoot,
    wall_contact_state: WallContactState,
    health: Health,
    input_angle: InputAngle,
}

impl Default for BasePlayerBundle {
    fn default() -> Self {
        Self {
            velocity: Velocity(Vec2::ZERO),
            can_shoot: CanShoot {
                value: true,
                since_last: 999,
            },
            wall_contact_state: WallContactState::default(),
            health: Health(1),
            input_angle: InputAngle(0),
        }
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    base: BasePlayerBundle,
    sprite: SpriteSheetBundle,
    facing: Facing,
    hitbox: Hitbox,
    animation: AnimationBundle,
    wall_sensors: WallSensors,
    points: Points,
}

impl PlayerBundle {
    pub fn new(id: usize, atlas: Handle<TextureAtlas>) -> Self {
        const SIZE: f32 = 4.1;
        const E: f32 = 0.05;
        Self {
            base: BasePlayerBundle::default(),
            player: Player { id },
            sprite: SpriteSheetBundle {
                texture_atlas: atlas,
                sprite: TextureAtlasSprite::new(0),
                ..default()
            },
            facing: Facing::Down,
            animation: AnimationBundle::new(
                PlayerAnimation::Stand(Facing::Down).into(),
                Timer::from_seconds(0.125, TimerMode::Repeating),
            ),
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
            points: Points(0),
        }
    }
}

#[derive(Bundle)]
pub struct BowBundle {
    bow: Bow,
    sprite: SpriteSheetBundle,
    animation: AnimationBundle,
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
            animation: AnimationBundle::new(
                BowAnimation::Draw.into(),
                Timer::from_seconds(0.1, TimerMode::Once),
            ),
        }
    }
}
