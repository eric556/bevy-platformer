use bevy::{core::Timer, prelude::{Changed, Query}};
use macros::animation_graph;

use crate::animation::{Col, Row, SpriteSheetDefinition};


animation_graph!(
    Player,
    {}, // No resources needed
    {vel: crate::physics::body::Velocity},
    Jump {
		Fall -> vel.0.y <= 0.0,
	},
	Fall {
		Idle -> vel.0.y == 0.0,
        Jump -> vel.0.y > 0.0
	},
	Idle {
		Jump -> vel.0.y != 0.0 && vel.0.y > 0.0,
		Fall -> vel.0.y != 0.0 && vel.0.y < 0.0,
		Run ->  vel.0.x != 0.0
	},
	Run {
		Jump -> vel.0.y != 0.0 && vel.0.y > 0.0,
		Fall -> vel.0.y != 0.0 && vel.0.y < 0.0,
		Idle -> vel.0.x == 0.0
	}
);

impl Default for Player::PlayerAnimationUpdate {
    fn default() -> Self {
        Self::Idle
    }
}

pub fn update_player_animation(
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