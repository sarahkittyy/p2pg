use bevy::prelude::*;
use bevy_ggrs::*;
use bytemuck::{Pod, Zeroable};

use crate::component::{InputAngle, Player};

use std::f32::consts::PI;

#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct PlayerInput {
    pub btn: u8,
    pub angle: u8, // 0 - 255 clockwise from up
}

impl PlayerInput {
    pub fn fire(&self) -> bool {
        self.btn & FIRE != 0
    }

    pub fn direction(&self) -> Vec2 {
        let mut dir = Vec2::ZERO;

        if self.btn & LEFT != 0 {
            dir += Vec2::NEG_X;
        }
        if self.btn & RIGHT != 0 {
            dir += Vec2::X;
        }
        if self.btn & UP != 0 {
            dir += Vec2::Y;
        }
        if self.btn & DOWN != 0 {
            dir += Vec2::NEG_Y;
        }
        dir.normalize_or_zero()
    }
}

// user inputs
pub const UP: u8 = 1 << 0;
pub const DOWN: u8 = 1 << 1;
pub const LEFT: u8 = 1 << 2;
pub const RIGHT: u8 = 1 << 3;
pub const FIRE: u8 = 1 << 4;

// angle should be between 0 and 2 * PI
pub fn to_u8_angle(angle: f32) -> u8 {
    let t = angle / (2. * PI);
    let angle = t.clamp(0., 1.) * 255.;
    angle.floor() as u8
}

// 0-255 => 0-2pi
pub fn from_u8_angle(angle: u8) -> f32 {
    let t = (angle as f32) / 255.;
    let angle = t.clamp(0., 1.) * 2. * PI;
    angle
}

// angle clockwise from +Y from 0 to 2 PI
pub fn vec_to_angle(dir: Vec2) -> f32 {
    dir.angle_between(Vec2::NEG_Y) + PI
}

// angle is 0-2pi, returns unit vec clockwise from +y
pub fn angle_to_vec(angle: f32) -> Vec2 {
    angle.sin_cos().into()
}

pub fn input(
    player_handle: In<ggrs::PlayerHandle>,
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    q_window: Query<&Window>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut q_player: Query<(&Player, &mut InputAngle, &GlobalTransform)>,
) -> PlayerInput {
    let mut btn = 0u8;

    let window = q_window.single();
    let (camera, camera_transform) = q_camera.single();

    if keys.pressed(KeyCode::A) {
        btn |= LEFT;
    }
    if keys.pressed(KeyCode::D) {
        btn |= RIGHT;
    }
    if keys.pressed(KeyCode::W) {
        btn |= UP;
    }
    if keys.pressed(KeyCode::S) {
        btn |= DOWN;
    }
    if mouse_buttons.pressed(MouseButton::Left) {
        btn |= FIRE;
    }

    let mut angle = 0u8;
    // get the cursor position in the world
    let cursor_pos: Option<Vec2> = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor));

    // fetch our own player position
    for (player, mut input_angle, player_transform) in &mut q_player {
        if player.id == player_handle.0 {
            if let Some(cursor_pos) = cursor_pos {
                // cursor in window
                let self_pos = player_transform.translation().truncate();
                angle = to_u8_angle(vec_to_angle(cursor_pos - self_pos));
                // cache this known angle
                input_angle.0 = angle;
            } else {
                // if no cursor pos, use the last known angle
                angle = input_angle.0;
            }
            break;
        }
    }
    PlayerInput { btn, angle }
}
