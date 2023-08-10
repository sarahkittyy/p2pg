use super::AnimationIndices;

#[derive(Debug, Clone)]
pub enum BowAnimation {
    Empty,
    Draw,
}

impl From<BowAnimation> for AnimationIndices {
    fn from(anim: BowAnimation) -> Self {
        match anim {
            BowAnimation::Empty => AnimationIndices::from_range(0, 0),
            BowAnimation::Draw => AnimationIndices::from_range(1, 3),
        }
    }
}
