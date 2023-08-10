use bevy::prelude::*;

mod bow;
mod player;
pub use bow::*;
pub use player::*;
use tiled::Frame;

#[derive(Component, Clone, Reflect, Debug, Default, PartialEq, Eq)]
pub struct AnimationIndices {
    pub frames: Vec<usize>,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl AnimationIndices {
    #[inline]
    pub fn from_range(first: usize, last: usize) -> Self {
        Self {
            frames: (first..=last).collect(),
            flip_x: false,
            flip_y: false,
        }
    }

    pub fn from_frames(frames: &Vec<Frame>) -> Self {
        Self {
            frames: frames.iter().map(|f| f.tile_id as usize).collect(),
            flip_x: false,
            flip_y: false,
        }
    }

    #[inline]
    pub fn with_flip(self, x: bool, y: bool) -> Self {
        Self {
            flip_x: x,
            flip_y: y,
            ..self
        }
    }
}

#[derive(Component, Debug, Reflect)]
pub struct AnimationTimer(pub Timer);

#[derive(Component, Debug, Reflect)]
pub struct AnimationFrame(pub usize);

#[derive(Bundle)]
pub struct AnimationBundle {
    pub indices: AnimationIndices,
    pub timer: AnimationTimer,
    pub frame: AnimationFrame,
}

impl AnimationBundle {
    pub fn new(indices: AnimationIndices, timer: Timer) -> Self {
        Self {
            indices,
            timer: AnimationTimer(timer),
            frame: AnimationFrame(0),
        }
    }
}

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AnimationIndices>()
            .register_type::<AnimationTimer>()
            .add_systems(Update, (on_animation_change, tick_animations).chain());
    }
}

fn on_animation_change(
    mut query: Query<
        (
            &AnimationIndices,
            &mut AnimationFrame,
            &mut AnimationTimer,
            &mut TextureAtlasSprite,
        ),
        Changed<AnimationIndices>,
    >,
) {
    for (indices, mut frame, mut timer, mut sprite) in &mut query {
        timer.0.reset();
        frame.0 = 0;
        sprite.flip_x = indices.flip_x;
        sprite.flip_y = indices.flip_y;
        sprite.index = indices.frames[0];
    }
}

fn tick_animations(
    time: Res<Time>,
    mut query: Query<(
        &AnimationIndices,
        &mut AnimationFrame,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
    )>,
) {
    for (indices, mut frame, mut timer, mut sprite) in &mut query {
        let max_frame = indices.frames.len() - 1;
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            timer.0.reset();
            // increment the animation, with wraparound
            frame.0 = if frame.0 == max_frame {
                if timer.0.mode() == TimerMode::Once {
                    max_frame
                } else {
                    0
                }
            } else {
                frame.0 + 1
            }
            .clamp(0, max_frame);

            sprite.index = indices.frames[frame.0];
        }
    }
}
