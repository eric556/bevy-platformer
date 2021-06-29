use crate::kinematic::colliders::check_dynamic_box_box_intersection;
use bevy::{math::Vec2, prelude::*};
use bevy_canvas::{Canvas, DrawMode, common_shapes::{Circle, Line}};
use super::{ADD_ACCELERATION, ADD_VELOCITY, COLLISION_RESOLUTION, PHYSICS_UPDATE, colliders::{Collider}};


#[derive(Default)]
pub struct Position(pub Vec2);
#[derive(Default)]
pub struct Velocity(pub Vec2);

#[derive(Bundle, Default)]
pub struct KinematicBundle {
    pub position: Position,
    pub velocity: Velocity,
}

pub fn resolve_collisions(
    time: Res<Time>,
    mut canvas: ResMut<Canvas>,
    mut dynamics_query: Query<(&mut Velocity, &mut Position, &Collider)>,
    static_query: Query<(&Position, &Collider), Without<Velocity>>
) {
    for (mut velocity, mut position, dyn_collider) in dynamics_query.iter_mut() {
        for (stat_pos, stat_collider) in static_query.iter() {
            match dyn_collider {
                Collider::Box(dyn_box_collider) => {
                    match stat_collider {
                        Collider::Box(stat_box_collider) => {
                            if let Some(collision) = check_dynamic_box_box_intersection(
                                &dyn_box_collider.adjusted_position(&position.0), 
                                &velocity, 
                                &stat_box_collider.adjusted_position(&stat_pos.0), 
                                time.delta_seconds()
                            ) {
                                if collision.t <= 1.0 && collision.t >= 0.0 {
                                    // position.0 = collision.contact_point;
                                    let vel_adjustment = collision.contact_normal * velocity.0.abs() * (1.0 - collision.t);
                                    velocity.0 += vel_adjustment;
                                    canvas.draw(&Circle {
                                        center: collision.contact_point,
                                        radius: 5.0
                                    }, DrawMode::fill_simple(), Color::RED);

                                    // canvas.draw(&Line(collision.contact_point, collision.contact_point + vel_adjustment), DrawMode::stroke_1px(), Color::GREEN);
                                }
                                // velocity.0 = Vec2::ZERO;
                            }
                        },
                        Collider::Circle(_) => todo!(),
                    }
                },
                Collider::Circle(_) => todo!(),
            }
        }
    }
}

fn add_velocity(
    time: Res<Time>,
    mut kinematic_query: Query<(&mut Velocity, &mut Position)>,
) {
    for (mut vel, mut pos) in kinematic_query.iter_mut() {
        pos.0 += vel.0 * time.delta_seconds();
    }
}

fn apply_position_to_transform(
    mut transform_kinematic_query: Query<(&mut Transform, &Position)>
) {
    for (mut transform, position) in transform_kinematic_query.iter_mut() {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
    }
}

pub struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_system(resolve_collisions.system().chain(add_velocity.system()).label(PHYSICS_UPDATE));
        app.add_system(apply_position_to_transform.system().label("apply_transform").after(PHYSICS_UPDATE));
    }
}