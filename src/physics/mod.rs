use std::panic;

use bevy::{core::Time, math::{IVec2, Vec2}, prelude::{Color, Entity, IntoSystem, ParallelSystemDescriptorCoercion, Plugin, Query, QuerySet, Res, ResMut, Transform}, render::render_graph::SlotLabel};
use bevy_canvas::{Canvas, DrawMode, common_shapes::{Rectangle, RectangleAnchor}};
use self::{body::{BodyType, Position, Remainder, Velocity}, collision::{AABB, Intersection}};

pub mod collision;
pub mod body;

fn apply_body_position_to_transform(
    mut transform_body_query: Query<(&mut Transform, &Position)>
) {
    for (mut transform, position) in transform_body_query.iter_mut() {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
    }
}

fn check_for_collision(
    collider: &AABB,
    position: &Position,
    colliders: &Vec<(Position, AABB)>
) -> bool{

    for (other_position, other_collider) in colliders.iter() {
        let current_ent_pos = IVec2::new(position.0.x.round() as i32, position.0.y.round() as i32);
        let other_ent_pos = IVec2::new(other_position.0.x.round() as i32, other_position.0.y.round() as i32);

        // println!("Checking {:?} vs {:?}", collider.adjusted_position(&current_ent_pos), other_collider.adjusted_position(&other_ent_pos));

        if AABB::interescts(&collider.adjusted_position(&current_ent_pos), &other_collider.adjusted_position(&other_ent_pos)) {
            return true;
        }

    }

    return false;
}

fn move_actor(
    time: Res<Time>,
    mut stuff: QuerySet<(
        Query<(&mut Position, &mut Velocity, &mut Remainder, &AABB, &BodyType)>,
        Query<(&Position, &AABB, &BodyType)>
    )>
) {
    let solid_colliders: Vec<(Position, AABB)> = stuff.q1().iter().filter(|(position, aabb, body_type)| {
        **body_type == BodyType::Solid
    }).map(|(position, aabb, body_type)| {
        (*position, *aabb)
    }).collect();

    for (mut position, mut velocity, mut remainder, collider, body_type) in stuff.q0_mut().iter_mut() {
        if *body_type == BodyType::Actor {
            // println!("Start Pos({:?})", position);
            // Move X
            remainder.0.x += velocity.0.x;
            let mut movement: i32 = remainder.0.x.round() as i32;

            if movement != 0i32 {
                remainder.0.x -= movement as f32;
                let sign = movement.signum();
                while movement != 0i32 {
                    let next = Position(position.0 + Vec2::new(sign as f32, 0.0));
                    if !check_for_collision( &collider, &next, &solid_colliders) {
                        position.0.x += sign as f32;
                        movement -= sign;
                    } else {
                        // STOP WE HIT SOMETHING
                        break;  
                    }
                }
            }

            // Move Y
            remainder.0.y += velocity.0.y;
            movement = remainder.0.y.round() as i32;

            if movement != 0i32 {
                remainder.0.y -= movement as f32;
                let sign = movement.signum();
                while movement != 0i32 {
                    let next = Position(position.0 + Vec2::new(0.0, sign as f32));
                    if !check_for_collision(&collider, &next , &solid_colliders) {
                        position.0.y += sign as f32;
                        movement -= sign;
                    } else {
                        // STOP WE HIT SOMETHING
                        break;  
                    }
                }
            }

            
            // println!("End Pos({:?})", position);
            velocity.0 = Vec2::ZERO;
        }

    }
}

fn debug_aabb(
    mut canvas: ResMut<Canvas>,
    aabb_qery: Query<(&Position, &AABB, &BodyType)>,
) {
    for (position, aabb, body_type) in aabb_qery.iter() {
        let temp_extents = aabb.half_size * 2i32;
        let color = if *body_type == BodyType::Actor { Color::GREEN } else { Color::RED };
        canvas.draw(&Rectangle {
            origin: position.0 + Vec2::new(aabb.position.x as f32, aabb.position.y as f32),
            extents: Vec2::new(temp_extents.x as f32, temp_extents.y as f32),
            anchor_point: RectangleAnchor::Center
        }, DrawMode::stroke_1px(), color);
    }
}

pub struct DebugAABBPlugin;

impl Plugin for DebugAABBPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_system(debug_aabb.system());
    }
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_system(move_actor.system().label("MOVE_ACTORS"))
        .add_system(apply_body_position_to_transform.system().after("MOVE_ACTORS"));
    }
} 