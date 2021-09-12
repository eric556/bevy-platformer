use bevy::{core::{Time, Timer}, prelude::{AppBuilder, Bundle, IntoSystem, Plugin, Query, Res, SpriteSheetBundle}, sprite::TextureAtlasSprite};

#[derive(Default)]
pub struct AnimationDefinition {
    pub name: String,
    pub number_of_frames: usize,
    pub frame_time: f32,
    pub repeating: bool
}

#[derive(Default)]
pub struct SpriteSheetDefinition {
    pub animation_definitions: Vec<AnimationDefinition>,
    pub rows: usize,
    pub columns: usize
} 

#[derive(Default)]
pub struct Row(pub usize);

#[derive(Default)]
pub struct Col(pub usize);

#[derive(Bundle, Default)]
pub struct AnimatedSpriteBundle {
    #[bundle]
    pub sprite_sheet: SpriteSheetBundle,
    pub sprite_sheet_definitions: SpriteSheetDefinition,
    pub current_row: Row,
    pub current_col: Col,
    pub animation_timer: Timer,
}

fn animate_sprite_system(
    time: Res<Time>,
    mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &SpriteSheetDefinition, &Row, &mut Col)>,
) {
    for (mut timer, mut sprite, sheet_def, row, mut col) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            col.0 += 1;
            if row.0 < sheet_def.rows {
                let columns = sheet_def.animation_definitions[row.0].number_of_frames;
                if col.0 >= columns{
                    col.0 = 0;
                }
            }
            sprite.index = (col.0 + sheet_def.columns * row.0) as u32;
        }
    }
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(animate_sprite_system.system());
    }
}
