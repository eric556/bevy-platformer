use bevy::math::{IVec2, Vec2};

use super::body::BodyBundle;

#[derive(Default, Clone, Copy, Debug)]
pub struct AABB {
    pub position: IVec2,
    pub half_size: IVec2
}

pub struct Collision {
    pub position: Vec2,
    pub collider: AABB
}

pub struct CollisionResult {
    pub x_collision_body: Option<Collision>,
    pub y_collision_body: Option<Collision>
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

pub fn check_for_collision(
    collider: &AABB,
    position: &Vec2,
    colliders: &Vec<(Vec2, AABB)>
) -> Option<Collision> {

    for (other_position, other_collider) in colliders.iter() {
        let current_ent_pos = IVec2::new(position.x.round() as i32, position.y.round() as i32);
        let other_ent_pos = IVec2::new(other_position.x.round() as i32, other_position.y.round() as i32);

        if AABB::interescts(&collider.adjusted_position(&current_ent_pos), &other_collider.adjusted_position(&other_ent_pos)) {
            return Some(Collision {
                position: *other_position,
                collider: collider.clone(),
            });
        }
    }

    return None;
}