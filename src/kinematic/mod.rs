use bevy::{math::Vec2, prelude::*};

#[derive(Default)]
pub struct Position(pub Vec2);
#[derive(Default)]
pub struct Velocity(pub Vec2);
#[derive(Default)]
pub struct Acceleration(pub Vec2);
#[derive(Default)]
pub struct Dynamic;
pub struct Static;

#[derive(Bundle, Default)]
pub struct KinematicBundle {
    pub position: Position,
    pub velocity: Velocity,
    pub acceleration: Acceleration
}

fn update_kinematics(
    time: Res<Time>,
    mut kinematic_query: Query<(&mut Velocity, &mut Position, &mut Acceleration)>
) {
    for (mut vel, mut pos, mut accel) in kinematic_query.iter_mut() {
        vel.0.x += accel.0.x;
        vel.0.y += accel.0.y;

        accel.0.x = 0f32;
        accel.0.y = 0f32;

        pos.0.x += vel.0.x * time.delta_seconds();
        pos.0.y += vel.0.y * time.delta_seconds();
    }
}

fn apply_kinematics_to_transform(
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
        app
        .add_system(update_kinematics.system())
        .add_system(apply_kinematics_to_transform.system());
    }
}