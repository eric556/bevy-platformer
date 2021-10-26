use std::panic;

use bevy::{core::{FixedTimestep, FixedTimesteps, Time}, math::{IVec2, Vec2}, prelude::{Color, CoreStage, Entity, IntoSystem, ParallelSystemDescriptorCoercion, Plugin, Query, QuerySet, Res, ResMut, StageLabel, SystemStage, Transform}, render::render_graph::SlotLabel};
use bevy_canvas::{Canvas, DrawMode, common_shapes::{Rectangle, RectangleAnchor}};
use self::{body::{Acceleration, BodyType, Position, Remainder, Velocity}, collision::{AABB, Intersection}};

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

fn move_x(
    position: &mut Position, 
    velocity: &mut Velocity, 
    remainder: &mut Remainder, 
    collider: &AABB,
    solid_colliders: &Vec<(Position, AABB)>,
    time: &Time
) {
    remainder.0.x += velocity.0.x * time.delta_seconds();
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
                velocity.0.x = 0.0;
                // STOP WE HIT SOMETHING
                break;  
            }
        }
    }
}

fn move_y(
    position: &mut Position, 
    velocity: &mut Velocity, 
    remainder: &mut Remainder, 
    collider: &AABB,
    solid_colliders: &Vec<(Position, AABB)>,
    time: &Time
) {
    remainder.0.y += velocity.0.y * time.delta_seconds();
    let mut movement: i32 = (velocity.0.y * time.delta_seconds()).round() as i32;
    // println!("{:?}", remainder);

    if movement != 0i32 {
        remainder.0.y -= movement as f32;
        let sign = movement.signum();
        while movement != 0i32 {
            let next = Position(position.0 + Vec2::new(0.0, sign as f32));
            if !check_for_collision(&collider, &next , &solid_colliders) {
                position.0.y += sign as f32;
                movement -= sign;
            } else {
                velocity.0.y = 0.0;
                // STOP WE HIT SOMETHING
                break;  
            }
        }

        // velocity.0 = Vec2::ZERO;
    }
}

fn move_actor(
    time: Res<Time>,
    fixed_timesteps: Res<FixedTimesteps>,
    mut stuff: QuerySet<(
        Query<(&mut Position, &mut Velocity, &mut Acceleration, &mut Remainder, &AABB, &BodyType)>,
        Query<(&Position, &AABB, &BodyType)>
    )>
) {
    let solid_colliders: Vec<(Position, AABB)> = stuff.q1().iter().filter(|(_, _, body_type)| {
        **body_type == BodyType::Solid
    }).map(|(position, aabb, _)| {
        (*position, *aabb)
    }).collect();

    // let dt = fixed_timesteps.get("FIXED_TIME_STEP").unwrap();
    let mut i = 0;
    for (mut position, mut velocity, mut acceleration, mut remainder, collider, body_type) in stuff.q0_mut().iter_mut() {
        
        if *body_type == BodyType::Actor {
            let start_position = position.0;
            move_x(
                &mut position, 
                &mut velocity, 
                &mut remainder, 
                collider, 
                &solid_colliders, 
                &time
            );

            move_y(
                &mut position, 
                &mut velocity, 
                &mut remainder, 
                collider, 
                &solid_colliders, 
                &time
            );

            velocity.1 = position.0 - start_position;
            println!("Vel({:?}), Actual({:?})", velocity.0 * time.delta_seconds(), velocity.1);
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum PhysicsStages {
    PreStep,
    Step,
    PostStep
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app
        .add_stage_before(
            CoreStage::Update, 
            PhysicsStages::Step, 
            SystemStage::parallel()
            // .with_run_criteria(
            //     FixedTimestep::step(1.0 / 60.0).with_label("FIXED_TIME_STEP")
            // )
            .with_system(move_actor.system().label("MOVE_ACTORS")))
            .add_stage_before(PhysicsStages::Step, PhysicsStages::PreStep, SystemStage::parallel())
            .add_stage_after(PhysicsStages::Step, PhysicsStages::PostStep, SystemStage::parallel().with_system(apply_body_position_to_transform.system()));
    }
}  