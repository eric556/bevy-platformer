use bevy::{core::{FixedTimestep, FixedTimesteps, Time}, math::{IVec2, Vec2}, prelude::{Color, CoreStage, Entity, IntoSystem, ParallelSystemDescriptorCoercion, Plugin, Query, QuerySet, Res, ResMut, StageLabel, SystemStage, SystemLabel, Transform}};
use bevy_canvas::{Canvas, DrawMode, common_shapes::{Rectangle, RectangleAnchor}};
use bevy_egui::{EguiContext, egui::Window};
use self::{body::{Acceleration, BodyParams, BodyType, Position, Remainder, Velocity}, collision::{AABB, Intersection}};

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

        if AABB::interescts(&collider.adjusted_position(&current_ent_pos), &other_collider.adjusted_position(&other_ent_pos)) {
            return true;
        }
    }

    return false;
}

fn move_x(
    move_amount: &f32,
    position: &mut Position, 
    remainder: &mut Remainder, 
    collider: &AABB,
    solid_colliders: &Vec<(Position, AABB)>,
) {
    remainder.0.x += move_amount;
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
}

fn move_y(
    move_amount: &f32,
    position: &mut Position, 
    remainder: &mut Remainder, 
    collider: &AABB,
    solid_colliders: &Vec<(Position, AABB)>,
) {
    // println!("Remainder {:?}", remainder);
    remainder.0.y += move_amount;
    let mut movement: i32 = remainder.0.y.round() as i32;

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

        // velocity.0 = Vec2::ZERO;
    }
}

fn move_actor(
    time: Res<Time>,
    mut stuff: QuerySet<(
        Query<(&mut Position, &mut Velocity, &mut Acceleration, &mut Remainder, &AABB, &BodyType, &BodyParams)>,
        Query<(&Position, &AABB, &BodyType)>
    )>
) {
    let solid_colliders: Vec<(Position, AABB)> = stuff.q1().iter().filter(|(_, _, body_type)| {
        **body_type == BodyType::Solid
    }).map(|(position, aabb, _)| {
        (*position, *aabb)
    }).collect();

    // let dt = fixed_timesteps.get("FIXED_TIME_STEP").unwrap();
    for (mut position, mut velocity, mut acceleration, mut remainder, collider, body_type, body_params) in stuff.q0_mut().iter_mut() {
        if *body_type == BodyType::Actor {
            let move_amount = velocity.0 * time.delta_seconds();
            let start_position = position.0;
            move_x(&move_amount.x, &mut position, &mut remainder, collider, &solid_colliders);
            move_y(&move_amount.y, &mut position, &mut remainder, collider, &solid_colliders);

            velocity.0 = (position.0 - start_position) / time.delta_seconds();
            acceleration.0 = Vec2::ZERO;
        }
    }
}

fn debug_body_information(
    mut egui_ctx: ResMut<EguiContext>,
    body_query: Query<(&Position, &Velocity, &Acceleration, &Remainder, &AABB, &BodyType)>,
) {
    Window::new("Bodies").scroll(true).show(egui_ctx.ctx(), |ui| {
        ui.collapsing("Actors", |ui| {
            let mut i = 0u32;
            for (pos, vel, accel, remain, aabb, _) in body_query.iter().filter(|(_, _, _, _, _, body_type)| { return **body_type == BodyType::Actor; }) {
                ui.collapsing(format!("Actor {}", i), |ui| {
                    ui.label(format!("Position: {:?}", pos));
                    ui.label(format!("Velocity: {:?}", vel));
                    ui.label(format!("Acceleration: {:?}", accel));
                    ui.label(format!("Remainder: {:?}", remain));
                    ui.label(format!("AABB: {:?}", aabb));
                });
                i += 1;
            }
        });

        ui.separator();

        ui.collapsing("Solids", |ui| {
            for (pos, vel, accel, remain, aabb, _) in body_query.iter().filter(|(_, _, _, _, _, body_type)| { return **body_type == BodyType::Solid; }) {
                ui.label(format!("Position: {:?}", pos));
                ui.label(format!("Velocity: {:?}", vel));
                ui.label(format!("Acceleration: {:?}", accel));
                ui.label(format!("Remainder: {:?}", remain));
                ui.label(format!("AABB: {:?}", aabb));
            }
        });
    });
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

pub struct DebugPhysicsPlugin;

impl Plugin for DebugPhysicsPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_system(debug_aabb.system());
        app.add_system_to_stage(PhysicsStages::PreStep, debug_body_information.system());
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum PhysicsStages {
    PreStep,
    Step,
    PostStep
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum StepSystemLabels {
    Integrate,
    MoveActors
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {

        // Step stages
        app.add_stage_before(
            CoreStage::Update,
             PhysicsStages::Step, 
             SystemStage::parallel()
            // .with_run_criteria(
            //     FixedTimestep::step(1.0 / 60.0).with_label("FIXED_TIME_STEP")
            // )
            .with_system(
                move_actor.system().label(StepSystemLabels::MoveActors)
            ));

        // Pre and post stages
        app.add_stage_before(PhysicsStages::Step, PhysicsStages::PreStep, SystemStage::parallel())
            .add_stage_after(PhysicsStages::Step, PhysicsStages::PostStep, SystemStage::parallel().with_system(apply_body_position_to_transform.system()));
    }
}  