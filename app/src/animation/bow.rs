use crate::component::AnimationIndices;

pub const BOW_EMPTY: AnimationIndices = AnimationIndices {
    first: 0,
    last: 0,
    flip_x: false,
    flip_y: false,
};

pub const BOW_DRAW: AnimationIndices = AnimationIndices {
    first: 1,
    last: 3,
    flip_x: false,
    flip_y: false,
};
