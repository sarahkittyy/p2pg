use bevy::prelude::*;

use super::AnimationIndices;
use crate::component::Facing;

#[derive(Debug, Clone)]
pub enum PlayerAnimation {
    Walk(Facing),
    Stand(Facing),
}

impl From<PlayerAnimation> for AnimationIndices {
    fn from(anim: PlayerAnimation) -> Self {
        use Facing::*;
        use PlayerAnimation::*;
        match anim {
            Walk(facing) => match facing {
                Down => AnimationIndices::from_range(0, 7),
                Left => AnimationIndices::from_range(8, 15),
                Right => AnimationIndices::from_range(8, 15).with_flip(true, false),
                Up => AnimationIndices::from_range(16, 23),
            },
            Stand(facing) => match facing {
                Down => AnimationIndices::from_range(0, 0),
                Left => AnimationIndices::from_range(8, 8),
                Right => AnimationIndices::from_range(8, 8).with_flip(true, false),
                Up => AnimationIndices::from_range(16, 16),
            },
        }
    }
}

/// compute animation indices and whether to flip x/y given the player's velocity and current orientation
pub fn player_animation_indices(vel: Vec2, facing: &Facing) -> AnimationIndices {
    // standing still
    if vel.length() < f32::EPSILON {
        PlayerAnimation::Stand(*facing)
    } else {
        PlayerAnimation::Walk(*facing)
    }
    .into()
}
