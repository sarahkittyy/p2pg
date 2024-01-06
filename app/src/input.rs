use bevy::{prelude::*, utils::HashMap};
use bevy_egui::EguiContexts;
use bevy_ggrs::*;
use bytemuck::{Pod, Zeroable};
use std::f32::consts::PI;

use crate::{
    component::{InputAngle, Player},
    p2p::GgrsConfig,
};

mod touch;
pub use touch::*;

#[derive(Clone, Copy, PartialEq, Zeroable, Pod, Debug, Default)]
#[repr(C)]
pub struct PlayerInput {
    pub dir: u8, // 0 - 255 clockwise from up
    pub btn: u8,
    pub angle: u8, // 0 - 255 clockwise from up
}

impl PlayerInput {
    pub fn fire(&self) -> bool {
        self.btn & FIRE != 0
    }

    pub fn moving(&self) -> bool {
        self.btn & MOVE != 0
    }

    pub fn direction(&self) -> Vec2 {
        angle_to_vec(from_u8_angle(self.dir)).normalize_or_zero()
    }
}

// button inputs
pub const MOVE: u8 = 1 << 0; // any move input (usually wasd on pc and touch on mobile)
pub const FIRE: u8 = 1 << 1;

/// convert a 2d coordinate from view space to world space
pub fn view_to_world(pos: Vec2, camera: &Camera, transform: &Transform) -> Vec2 {
    camera
        .viewport_to_world_2d(&GlobalTransform::default().mul_transform(*transform), pos)
        .expect("Could not transform position from world to view space.")
}

// angle should be between 0 and 2 * PI
pub fn to_u8_angle(angle: f32) -> u8 {
    let t = angle / (2. * PI);
    let angle = t.clamp(0., 1.) * 255.;
    angle.round() as u8
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

/// primary ggrs input system
pub fn input(
    mut commands: Commands,
    local_players: Res<LocalPlayers>,
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut touch: ResMut<TouchMovement>,

    q_window: Query<(Entity, &Window)>,
    mut q_player: Query<(&Player, &mut InputAngle)>,

    mut ctxs: EguiContexts,
) {
    let mut local_inputs = HashMap::new();

    for handle in &local_players.0 {
        let (window_entity, window) = q_window.single();

        let ctx = ctxs.try_ctx_for_window_mut(window_entity);
        let Some((_, mut input_angle)) =
            q_player.iter_mut().filter(|(p, ..)| p.id == *handle).next()
        else {
            local_inputs.insert(*handle, PlayerInput::default());
            continue;
        };

        let window_size = Vec2::new(window.width(), window.height());

        if let Some(touch_input) = touch.drain(input_angle.0, window_size) {
            input_angle.0 = touch_input.angle;
            local_inputs.insert(*handle, touch_input);
            continue;
        }

        let mut btn = 0u8;
        let mut dir = IVec2::ZERO;
        if window.focused {
            if keys.pressed(KeyCode::A) {
                dir += IVec2::NEG_X;
            }
            if keys.pressed(KeyCode::D) {
                dir += IVec2::X;
            }
            if keys.pressed(KeyCode::W) {
                dir += IVec2::Y;
            }
            if keys.pressed(KeyCode::S) {
                dir += IVec2::NEG_Y;
            }
            if mouse_buttons.pressed(MouseButton::Left)
                && !ctx.is_some_and(|v| v.is_pointer_over_area())
            {
                btn |= FIRE;
            }
        }
        if dir.length_squared() != 0 {
            btn |= MOVE;
        }

        let mut angle = input_angle.0;
        // get the cursor position in the world
        let cursor_pos: Option<Vec2> = window.cursor_position();

        // fetch our own player position
        if let Some(cursor_pos) = cursor_pos {
            // cursor in window
            let center = window_size / 2.;
            let mut dir = cursor_pos - center;
            dir.y = -dir.y;
            angle = to_u8_angle(vec_to_angle(dir));
        }
        input_angle.0 = angle;

        let input = PlayerInput {
            dir: to_u8_angle(vec_to_angle(dir.as_vec2())),
            btn,
            angle,
        };

        local_inputs.insert(*handle, input);
    }

    commands.insert_resource(LocalInputs::<GgrsConfig>(local_inputs));
}
