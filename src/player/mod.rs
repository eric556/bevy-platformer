use bevy::prelude::*;
use bevy_canvas::{
    common_shapes::{self, Rectangle},
    Canvas, DrawMode,
};
use bevy_rapier2d::{
    physics::{
        ColliderBundle, QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
        RapierConfiguration, RigidBodyBundle,
    },
    prelude::{
        ColliderPosition, ColliderShape, Cuboid, InteractionGroups, QueryPipeline, RigidBodyForces,
        RigidBodyMassProps, RigidBodyVelocity,
    },
};

use crate::{GROUND_GROUP, SHAPE_CAST_GROUP, animation::{AnimatedSpriteBundle, Col, Row, SpriteSheetDefinition}};
use macros::animation_graph;

animation_graph!(
    Player,
    {vel: bevy_rapier2d::rapier::dynamics::RigidBodyVelocity},
    Jump {
		Fall -> vel.linvel.y < 0.0
	},
	Fall {
		Idle -> vel.linvel.y == 0.0
	},
	Idle {
		Jump -> vel.linvel.y != 0.0 && vel.linvel.y > 0.0,
		Fall -> vel.linvel.y != 0.0 && vel.linvel.y < 0.0,
		Run ->  vel.linvel.x != 0.0
	},
	Run {
		Jump -> vel.linvel.y != 0.0 && vel.linvel.y > 0.0,
		Fall -> vel.linvel.y != 0.0 && vel.linvel.y < 0.0,
		Idle -> vel.linvel.x == 0.0
	}
);

impl Default for Player::PlayerAnimationUpdate {
    fn default() -> Self {
        Self::Idle
    }
}

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
    #[bundle]
    pub rigid_body: RigidBodyBundle,
    #[bundle]
    pub collider: ColliderBundle,
    #[bundle]
    pub animation: AnimatedSpriteBundle,
    pub input: PlayerInput,
    pub state: PlayerState,
    pub action: Player::PlayerAnimationUpdate,
    pub player_stats: PlayerStats,
}

fn update_player_animation(
    mut player_query: Query<
        (
            &Player::PlayerAnimationUpdate,
            &SpriteSheetDefinition,
            &mut Timer,
            &mut Row,
            &mut Col
        ),
        Changed<Player::PlayerAnimationUpdate>,
    >,
) {
    for (player_action, sprite_sheet_def, mut timer, mut row, mut col) in player_query.iter_mut()
    {
        row.0 = match player_action {
            Player::PlayerAnimationUpdate::Idle => 5,
            Player::PlayerAnimationUpdate::Run => 1,
            Player::PlayerAnimationUpdate::Fall => 6,
            Player::PlayerAnimationUpdate::Jump => 7,
            _ => todo!("Implement rest of player state animations"),
        };

        // reset the timer
        let def = &sprite_sheet_def.animation_definitions[row.0];
        *timer = Timer::from_seconds(def.frame_time, def.repeating);

        // reset to begining of animation
        col.0 = 0;
    }
}

fn update_player_grounded(
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
    rapier_params: Res<RapierConfiguration>,
    mut canvas: ResMut<Canvas>,
    mut player_query: Query<(&ColliderPosition, &ColliderShape, &mut PlayerState)>,
) {
    let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

    for (col_pos, col_shape, mut state) in player_query.iter_mut() {
        let bounds = col_shape.compute_aabb(&col_pos.0);
        let bounds_half_extents: Vec2 = bounds.half_extents().into();
        let collider_position = Vec2::from(col_pos.translation);
        // same width, half the hight of the bounding boxs
        let shape_down_up_width_adjust = 0.1 * bounds_half_extents.x;
        let shape_down_up = Cuboid::new(
            Vec2::new(
                bounds_half_extents.x - shape_down_up_width_adjust,
                bounds_half_extents.y / 4.0,
            )
            .into(),
        );
        let shape_down_pos = [
            collider_position.x,
            collider_position.y - (bounds_half_extents.y),
        ]
        .into();
        let shape_vel = Vec2::new(0.0, -0.01).into();
        let max_toi = 4.0;
        let groups = InteractionGroups::new(SHAPE_CAST_GROUP, GROUND_GROUP);
        let filter = None;

        if let Some((handle, hit)) = query_pipeline.cast_shape(
            &collider_set,
            &shape_down_pos,
            &shape_vel,
            &shape_down_up,
            max_toi,
            groups,
            filter,
        ) {
            // The first collider hit has the handle `handle`. The `hit` is a
            // structure containing details about the hit configuration.
            // println!("Hit the entity with the configuration: {:?}", hit);
            state.grounded = true;
        } else {
            state.grounded = false;
        }

        canvas.draw(
            &Rectangle {
                origin: Vec2::from(shape_down_pos.translation) * rapier_params.scale,
                extents: Vec2::from(shape_down_up.half_extents) * rapier_params.scale * 2.0,
                anchor_point: common_shapes::RectangleAnchor::Center,
            },
            DrawMode::stroke_1px(),
            Color::BLUE,
        );
    }
}

fn move_player(
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(
        &PlayerInput,
        &PlayerStats,
        &PlayerState,
        &mut RigidBodyVelocity
    )>,
) {
    for (p_input, player_stats, state, mut vel) in
        player_query.iter_mut()
    {
        let prev_vel_sign = vel.linvel.x.signum();

        if (!keys.pressed(p_input.left) && !keys.pressed(p_input.right))
            || (keys.pressed(p_input.left) && keys.pressed(p_input.right))
        {
            vel.linvel.x = 0.0;
        } else if keys.pressed(p_input.left) {
            if prev_vel_sign > 0.0 {
                vel.linvel.x = 0.0;
            }
            vel.linvel.x -= player_stats.speed_up;
            vel.linvel.x = vel.linvel.x.max(-player_stats.max_run_speed);
        } else if keys.pressed(p_input.right) {
            if prev_vel_sign < 0.0 {
                vel.linvel.x = 0.0;
            }
            vel.linvel.x += player_stats.speed_up;
            vel.linvel.x = vel.linvel.x.min(player_stats.max_run_speed);
        }

        if keys.pressed(p_input.jump) && state.grounded {
            if vel.linvel.y < 40.0 {
                vel.linvel.y += 5.0;
            }
            // vel.apply_impulse(mass, Vec2::new(0.0, 10.0).into());
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(move_player.system())
            .add_system(update_player_animation.system())
            .add_system(update_player_grounded.system())
            // .add_system(update_player_action.system());
            .add_system(Player::player_animation_update.system());
    }
}
