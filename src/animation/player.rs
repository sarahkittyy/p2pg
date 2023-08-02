use crate::component::{AnimationIndices, Facing};
use bevy::prelude::*;

pub const PLAYER_WALK_DOWN: AnimationIndices = AnimationIndices {
    first: 0,
    last: 7,
    flip_x: false,
    flip_y: false,
};
pub const PLAYER_WALK_LEFT: AnimationIndices = AnimationIndices {
    first: 8,
    last: 15,
    flip_x: false,
    flip_y: false,
};
pub const PLAYER_WALK_RIGHT: AnimationIndices = AnimationIndices {
    first: 8,
    last: 15,
    flip_x: true,
    flip_y: false,
};
pub const PLAYER_WALK_UP: AnimationIndices = AnimationIndices {
    first: 16,
    last: 23,
    flip_x: false,
    flip_y: false,
};
pub const PLAYER_STAND_DOWN: AnimationIndices = AnimationIndices {
    first: 0,
    last: 0,
    flip_x: false,
    flip_y: false,
};
pub const PLAYER_STAND_LEFT: AnimationIndices = AnimationIndices {
    first: 8,
    last: 8,
    flip_x: false,
    flip_y: false,
};
pub const PLAYER_STAND_RIGHT: AnimationIndices = AnimationIndices {
    first: 8,
    last: 8,
    flip_x: true,
    flip_y: false,
};
pub const PLAYER_STAND_UP: AnimationIndices = AnimationIndices {
    first: 16,
    last: 16,
    flip_x: false,
    flip_y: false,
};

/// compute animation indices and whether to flip x/y given the player's velocity and current orientation
pub fn player_animation_indices(vel: Vec2, facing: &Facing) -> AnimationIndices {
    // standing still
    if vel.length() < f32::EPSILON {
        match facing {
            Facing::Down => PLAYER_STAND_DOWN,
            Facing::Left => PLAYER_STAND_LEFT,
            Facing::Right => PLAYER_STAND_RIGHT,
            Facing::Up => PLAYER_STAND_UP,
        }
    } else {
        match facing {
            Facing::Down => PLAYER_WALK_DOWN,
            Facing::Left => PLAYER_WALK_LEFT,
            Facing::Right => PLAYER_WALK_RIGHT,
            Facing::Up => PLAYER_WALK_UP,
        }
    }
}
