use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub id: usize,
}

#[derive(Component, Reflect, Default)]
pub struct CanShoot(pub bool);

#[derive(Component)]
pub struct Bullet {
    pub dir: Vec2,
    pub vel: f32,
}