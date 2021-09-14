use bevy::prelude::*;
use bevy_canvas::{
    common_shapes::{self, Rectangle},
    Canvas, DrawMode,
};
use heron::{Acceleration, CollisionLayers, CollisionShape, PhysicsSystem, RigidBody, Velocity};

use crate::{GROUND_GROUP, SHAPE_CAST_GROUP, animation::{AnimatedSpriteBundle, Col, Row, SpriteSheetDefinition}};

#[derive(Default)]
pub struct Health(pub u32);

pub struct PlayerInput {
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub crouch: KeyCode,
}

impl Default for PlayerInput {
    fn default() -> Self {
        PlayerInput {
            left: KeyCode::A,
            right: KeyCode::D,
            jump: KeyCode::Space,
            crouch: KeyCode::S,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum PlayerAction {
    Idle,
    Running,
    Jumping,
    Falling,
}

impl Default for PlayerAction {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct PlayerState {
    pub grounded: bool,
}

#[derive(Default)]
pub struct PlayerStats {
    pub max_run_speed: f32,
    pub speed_up: f32,
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub health: Health,
    pub rigid_body_type: RigidBody,
    pub velocity: Velocity,
    pub collision_shape: CollisionShape,
    pub collision_layers: CollisionLayers,
    #[bundle]
    pub animation: AnimatedSpriteBundle,
    pub input: PlayerInput,
    pub state: PlayerState,
    pub action: PlayerAction,
    pub player_stats: PlayerStats,
}

fn update_player_animation(
    mut player_query: Query<
        (
            &PlayerAction,
            &SpriteSheetDefinition,
            &mut Timer,
            &mut Row,
            &mut Col
        ),
        Changed<PlayerAction>,
    >,
) {
    for (player_action, sprite_sheet_def, mut timer, mut row, mut col) in player_query.iter_mut()
    {
        row.0 = match player_action {
            PlayerAction::Idle => 5,
            PlayerAction::Running => 1,
            PlayerAction::Falling => 6,
            PlayerAction::Jumping => 7,
            _ => todo!("Implement rest of player state animations"),
        };

        // reset the timer
        let def = &sprite_sheet_def.animation_definitions[row.0];
        *timer = Timer::from_seconds(def.frame_time, def.repeating);

        // reset to begining of animation
        col.0 = 0;
    }
}

fn update_player_action(mut player_query: Query<(&Velocity, &mut PlayerAction)>) {
    for (vel, mut player_action) in player_query.iter_mut() {
        match *player_action {
            PlayerAction::Idle => {
                if vel.linear.y != 0.0 {
                    if vel.linear.y.signum() > 0.0 {
                        *player_action = PlayerAction::Jumping;
                    } else if vel.linear.y.signum() < 0.0 {
                        *player_action = PlayerAction::Falling;
                    }
                }

                if vel.linear.x != 0.0 {
                    *player_action = PlayerAction::Running;
                }
            }
            PlayerAction::Running => {
                if vel.linear.y != 0.0 {
                    if vel.linear.y.signum() > 0.0 {
                        *player_action = PlayerAction::Jumping;
                    } else if vel.linear.y.signum() < 0.0 {
                        *player_action = PlayerAction::Falling;
                    }
                }

                if vel.linear.x == 0.0 {
                    *player_action = PlayerAction::Idle;
                }
            }
            PlayerAction::Jumping => {
                if vel.linear.y == 0.0 {
                    if vel.linear.x != 0.0 {
                        *player_action = PlayerAction::Running;
                    } else {
                        *player_action = PlayerAction::Idle;
                    }
                } else if vel.linear.y.signum() < 0.0 {
                    *player_action = PlayerAction::Falling;
                }
            }
            PlayerAction::Falling => {
                if vel.linear.y == 0.0 {
                    if vel.linear.x != 0.0 {
                        *player_action = PlayerAction::Running;
                    } else {
                        *player_action = PlayerAction::Idle;
                    }
                } else if vel.linear.y.signum() > 0.0 {
                    *player_action = PlayerAction::Jumping;
                }
            }
        }
    }
}

// fn update_player_grounded(
//     query_pipeline: Res<QueryPipeline>,
//     collider_query: QueryPipelineColliderComponentsQuery,
//     rapier_params: Res<RapierConfiguration>,
//     mut canvas: ResMut<Canvas>,
//     mut player_query: Query<(&ColliderPosition, &ColliderShape, &mut PlayerState)>,
// ) {
//     let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

//     for (col_pos, col_shape, mut state) in player_query.iter_mut() {
//         let bounds = col_shape.compute_aabb(&col_pos.0);
//         let bounds_half_extents: Vec2 = bounds.half_extents().into();
//         let collider_position = Vec2::from(col_pos.translation);
//         // same width, half the hight of the bounding boxs
//         let shape_down_up_width_adjust = 0.1 * bounds_half_extents.x;
//         let shape_down_up = Cuboid::new(
//             Vec2::new(
//                 bounds_half_extents.x - shape_down_up_width_adjust,
//                 bounds_half_extents.y / 4.0,
//             )
//             .into(),
//         );
//         let shape_down_pos = [
//             collider_position.x,
//             collider_position.y - (bounds_half_extents.y),
//         ]
//         .into();
//         let shape_vel = Vec2::new(0.0, -0.01).into();
//         let max_toi = 4.0;
//         let groups = InteractionGroups::new(SHAPE_CAST_GROUP, GROUND_GROUP);
//         let filter = None;

//         if let Some((handle, hit)) = query_pipeline.cast_shape(
//             &collider_set,
//             &shape_down_pos,
//             &shape_vel,
//             &shape_down_up,
//             max_toi,
//             groups,
//             filter,
//         ) {
//             // The first collider hit has the handle `handle`. The `hit` is a
//             // structure containing details about the hit configuration.
//             // println!("Hit the entity with the configuration: {:?}", hit);
//             state.grounded = true;
//         } else {
//             state.grounded = false;
//         }

//         canvas.draw(
//             &Rectangle {
//                 origin: Vec2::from(shape_down_pos.translation) * rapier_params.scale,
//                 extents: Vec2::from(shape_down_up.half_extents) * rapier_params.scale * 2.0,
//                 anchor_point: common_shapes::RectangleAnchor::Center,
//             },
//             DrawMode::stroke_1px(),
//             Color::BLUE,
//         );
//     }
// }

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(
        &PlayerInput,
        &PlayerStats,
        &mut Velocity,
        &mut PlayerAction,
        &PlayerState,
    )>,
) {
    for (p_input, player_stats, mut vel, mut player_action, state) in
        player_query.iter_mut()
    {
        let prev_vel_sign = vel.linear.x.signum();

        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right))
            || (keys.pressed(p_input.left) && keys.pressed(p_input.right))
        {
            vel.linear.x = 0.0;
        } else if keys.pressed(p_input.left) {
            if prev_vel_sign > 0.0 {
                vel.linear.x = 0.0;
            }
            vel.linear.x -= player_stats.speed_up;
            vel.linear.x = vel.linear.x.max(-player_stats.max_run_speed);
        } else if keys.pressed(p_input.right) {
            if prev_vel_sign < 0.0 {
                vel.linear.x = 0.0;
            }
            vel.linear.x += player_stats.speed_up;
            vel.linear.x = vel.linear.x.min(player_stats.max_run_speed);
        }

        if keys.pressed(p_input.jump) && state.grounded {
            if vel.linear.y < 40.0 {
                vel.linear.y += 5.0;
            }
            // vel.apply_impulse(mass, Vec2::new(0.0, 10.0).into());
        }

        // println!("Linvel {:?}", vel.linear);
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(CoreStage::Update, move_player.system().label("MOVE_PLAYER"));
            // .add_system(update_player_animation.system().after("UPDATE_PLAYER_ACTION"))
            // .add_system(update_player_grounded.system())
            //  .add_system(update_player_action.system().label("UPDATE_PLAYER_ACTION"));
    }
}
