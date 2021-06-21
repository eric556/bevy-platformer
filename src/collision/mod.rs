use std::mem::swap;

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_canvas::{Canvas, DrawMode, common_shapes::{Rectangle, RectangleAnchor}};

use crate::kinematic::{Acceleration, Dynamic, Position, Velocity};
// TODO List
// [ ] Build quad tree for broad collision phase
// [ ] Seperating axis for polygon collision

// Components

#[derive(Default, Clone, Copy)]
pub struct AABB {
    pub position: Vec2,
    pub half_size: Vec2
}

trait Intersection<T> {
    fn interescts(_: &Self, _: &T) -> bool;
}

impl AABB {
    pub fn min(&self) -> Vec2 {
        return self.position - self.half_size;
    }

    pub fn max(&self) -> Vec2 {
        return self.position + self.half_size;
    }

    pub fn adjusted_position(&self, pos: &Vec2) -> Self {
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

// impl Intersection<Ray> for AABB {
//     fn interescts(&self, ray: Ray) -> bool {
//         let mut tmin: f32 = (self.min().x - ray.origin.x) / ray.direction.x;
//         let mut tmax: f32 = (self.max().x - ray.origin.x) / ray.direction.x;

//         if tmin > tmax {
//             swap(&mut tmin, &mut tmax);
//         }

//         let mut tymin: f32 = (self.min().y - ray.origin.y) / ray.direction.y;
//         let mut tymax: f32 = (self.max().y - ray.origin.y) / ray.direction.y;

//         if tymin > tymax {
//             swap(&mut tymin, &mut tymax);
//         }

//         return !((tmin > tymax) || (tymin > tmax));
//     }
// }

// pub struct Ray {
//     pub origin: Vec2,
//     pub direction: Vec2
// }

// impl Ray {
//     pub fn new(origin: Vec2, direction: Vec2) -> Self {

//     }
// }

pub struct Collision {
    pub entity_collided_with: Entity,
    pub collision_direction: Vec2,
    pub previous_velocity: Vec2,
    pub aabb_other: AABB
}

// Systems

fn narrow_phase(
    mut commands: Commands,
    dynamic_aabb_qery: Query<(&Position, &Velocity, &AABB, Entity)>,
    aabb_qery: Query<(&Position, &AABB, Entity)>
) {


    for (position, velocity, bounding_box, entity) in dynamic_aabb_qery.iter() {
        let current_adjusted_bounding_box = bounding_box.adjusted_position(&position.0);
        
        for (position_other, bounding_box_other, entity_other) in aabb_qery.iter() {
            if entity != entity_other {
                if AABB::interescts(
                    &current_adjusted_bounding_box, 
                    &bounding_box_other.adjusted_position(&position_other.0)
                ) {
                    commands.entity(entity).insert(
                        Collision { 
                            entity_collided_with: entity_other,
                            collision_direction: velocity.0.normalize_or_zero(),
                            previous_velocity: velocity.0,
                            aabb_other: bounding_box_other.clone()
                        }
                    );
                }
            }
        }
    }
}

fn collision_resolution(
    mut commands: Commands,
    mut collision_query: Query<(&Collision, &mut Position, &mut Velocity, &mut Acceleration, Entity), Added<Collision>>
) {
    for (collision, mut pos, mut vel, mut accel, entity) in collision_query.iter_mut() {
        println!("{:?} collided with {:?} going {:?}", entity, collision.entity_collided_with, collision.collision_direction);
        commands.entity(entity).remove::<Collision>();
    }
}

fn debug_aabb(
    mut canvas: ResMut<Canvas>,
    aabb_not_colliding_qery: Query<(&Position, &AABB), Without<Collision>>,
    aabb_colliding_qery: Query<(&Position, &AABB), With<Collision>>
) {
    for (position, aabb) in aabb_not_colliding_qery.iter() {
        canvas.draw(&Rectangle {
            origin: position.0 + aabb.position,
            extents: aabb.half_size * 2.0,
            anchor_point: RectangleAnchor::Center
        }, DrawMode::fill_simple(), Color::RED);
    }

    for (position, aabb) in aabb_colliding_qery.iter() {
        canvas.draw(&Rectangle {
            origin: position.0 + aabb.position,
            extents: aabb.half_size * 2.0,
            anchor_point: RectangleAnchor::Center
        }, DrawMode::fill_simple(), Color::GREEN);
    }
}

// Plugins

pub struct DebugCollisionPlugin;
pub struct CollisionPlugin;

impl Plugin for DebugCollisionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(debug_aabb.system());
    }
}

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(narrow_phase.system());
        app.add_system(collision_resolution.system());
    }
}