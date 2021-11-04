use bevy::{math::{Vec2, Vec3, Vec3Swizzles}, prelude::{IntoSystem, Plugin, Query, Transform, With, Without}};
use fastapprox::fast::ln;

use self::parallax::{move_parallax, parallax_start};

pub mod parallax;

pub struct MainCamera;
pub struct CameraTarget;

fn move_camera(
    target_query: Query<&Transform, With<CameraTarget>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<CameraTarget>)>,
) {
    let mut centorid = Vec2::ZERO;
    let mut n = 0.0;
    for transform in target_query.iter() {
        centorid += transform.translation.xy();
        n += 1.0;
    }
    centorid /= n;

    for mut transform in camera_query.iter_mut() {
        let distance = centorid.distance(transform.translation.xy());
        let z = transform.translation.z;

        // let k = 1.5f32;
        // let b = -0.02f32;
        // let a = -0.1f32;
        // let c = -0.8f32;
        // let mut t = (k / (1.0f32 + exp(a+(distance * b)))) + c;

        let g = 0.0004f32;
        let l = -1.0f32;
        let t = ln(g * distance - l);

        // if t < 0.01 {
        //     println!("Not moving {}", t.clamp(0.0, 1.0));
        //     transform.translation = Vec3::new(centorid.x, centorid.y, z);
        // } else {
        // println!("{}", t.clamp(0.0, 1.0));
        let new_position = transform.translation.xy().lerp(centorid, t.clamp(0.0, 1.0));
        transform.translation = Vec3::new(new_position.x, new_position.y, z);
        // }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_startup_system(parallax_start.system());
        app.add_system(move_parallax.system());
        app.add_system(move_camera.system());
    }
}