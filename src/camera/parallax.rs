use bevy::{math::Vec3Swizzles, prelude::*, render::camera::OrthographicProjection};
use bevy_egui::{EguiContext, egui};

use super::{CameraTarget, MainCamera};

#[derive(Default)]
pub struct ParallaxLayer {
    pub start_position: Vec3,
    pub parallax_factor: f32
}

pub fn parallax_start (
    mut layer_query: Query<(&Transform, &mut ParallaxLayer)>
) {
    for (transform, mut layer) in layer_query.iter_mut() {
        layer.start_position = transform.translation;
    }
}

pub fn move_parallax(
    mut egui_ctx: ResMut<EguiContext>,
    mut queries: QuerySet<(
        Query<&Transform, With<CameraTarget>>,
        Query<(&Transform, &OrthographicProjection), With<MainCamera>>,
        Query<(&mut Transform, &mut ParallaxLayer)>
    )>
) {
    if let Ok(camera_result) = queries.q1().single() {
        let camera_position = camera_result.0.translation;
        let near = camera_result.1.near;
        let far = camera_result.1.far;
        bevy_egui::egui::Window::new("Background").scroll(true).show(egui_ctx.ctx(), |ui| {
            ui.label(format!("Near: {}", near));
            ui.label(format!("Far: {}", far));
            let mut i = 0;
            egui::Grid::new(format!("BG {}", i)).show(ui, |ui|{
                for (mut layer_transform, mut layer) in queries.q2_mut().iter_mut() {
                    let travel = camera_position.xy() - layer.start_position.xy();
            
                    ui.label("Parallax Factor: ");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut layer.parallax_factor));
                    ui.end_row();
                    ui.label("Position: ");
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut layer_transform.translation.x));
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut layer_transform.translation.y));
                    ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut layer_transform.translation.z));
                    ui.end_row();

                    let new_pos = layer.start_position.xy() + travel * layer.parallax_factor;
                    layer_transform.translation.x = new_pos.x;
                    layer_transform.translation.y = new_pos.y;
                    i += 1;
                }
            });
        });

    }
}