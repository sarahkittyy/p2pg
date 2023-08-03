use crate::component::*;
use bevy::prelude::*;

mod bow;
mod player;
pub use bow::*;
pub use player::*;

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (on_animation_change, tick_animations).chain());
    }
}

fn on_animation_change(
    mut query: Query<
        (
            &AnimationIndices,
            &mut AnimationTimer,
            &mut TextureAtlasSprite,
        ),
        Changed<AnimationIndices>,
    >,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.0.reset();
        sprite.flip_x = indices.flip_x;
        sprite.flip_y = indices.flip_y;
        sprite.index = indices.first;
    }
}

fn tick_animations(
    time: Res<Time>,
    mut query: Query<(
        &AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
    )>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            timer.0.reset();
            // increment the animation, with wraparound
            sprite.index = if sprite.index == indices.last {
                if timer.0.mode() == TimerMode::Once {
                    indices.last
                } else {
                    indices.first
                }
            } else {
                sprite.index + 1
            }
            .clamp(indices.first, indices.last);
        }
    }
}
