use bevy::math::{IVec2, Vec2};

#[derive(Default, Clone, Copy, Debug)]
pub struct AABB {
    pub position: IVec2,
    pub half_size: IVec2
}

pub trait Intersection<T> {
    fn interescts(_: &Self, _: &T) -> bool;
}

impl AABB {
    pub fn min(&self) -> IVec2 {
        return self.position - self.half_size;
    }

    pub fn max(&self) -> IVec2 {
        return self.position + self.half_size;
    }

    pub fn adjusted_position(&self, pos: &IVec2) -> Self {
        AABB {
            position: self.position + *pos,
            half_size: self.half_size
        }
    }
}

impl Intersection<AABB> for AABB {
    fn interescts(box1: &Self, box2: &AABB) -> bool {
        return box1.min().x < box2.max().x &&
        box1.max().x > box2.min().x  &&
        box1.min().y < box2.max().y &&
        box1.max().y > box2.min().y;
    }
}