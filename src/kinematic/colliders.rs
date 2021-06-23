use std::{cmp::min, mem::swap, ops::Sub};

use bevy::{math::Vec4Swizzles, prelude::*};
use bevy_canvas::{Canvas, DrawMode, common_shapes::{Circle, Line, Rectangle, RectangleAnchor}};

use crate::MainCamera;

use super::kinematic::Position;

// Components

#[derive(Default, Clone, Copy)]
pub struct BoxCollider {
    pub position: Vec2,
    pub half_size: Vec2
}

#[derive(Default, Clone, Copy)]
pub struct CircleCollider {
    pub position: Vec2,
    pub radius: f32
}

pub struct Ray {
    pub origin: Vec2,
    pub direction: Vec2
}

pub struct RayCollision {
    pub contact_point: Vec2,
    pub contact_normal: Vec2,
    pub t: f32
}

impl Ray {
    pub fn new_from_points(start: Vec2, end: Vec2) -> Self {
        Ray {
            origin: start,
            direction: end - start
        }
    }
}

impl BoxCollider {
    pub fn min(&self) -> Vec2 {
        return self.position - self.half_size;
    }

    pub fn max(&self) -> Vec2 {
        return self.position + self.half_size;
    }

    pub fn adjusted_position(&self, pos: &Vec2) -> Self {
        BoxCollider {
            position: self.position + *pos,
            half_size: self.half_size
        }
    }
}

pub fn check_point_box_intersection(point: &Vec2, box_col: &BoxCollider) -> bool {
    return point.x >= box_col.min().x && point.y >= box_col.min().y && point.x < box_col.max().x && point.y < box_col.max().y;
}

pub fn check_box_box_intersection(box1: &BoxCollider, box2: &BoxCollider) -> bool {
    return box1.min().x < box2.max().x &&
           box1.max().x > box2.min().x  &&
           box1.min().y < box2.max().y &&
           box1.max().y > box2.min().y;
}

pub fn check_circle_circle_intersection(circle1: &CircleCollider, circle2: &CircleCollider) -> bool {
    let distance = circle1.position.distance(circle2.position);
    return distance < (circle1.radius + circle2.radius).powi(2)
}

pub fn check_box_circle_intersection(box_col: &BoxCollider, circle_col: &CircleCollider) -> bool {
    let diff = circle_col.position - box_col.position;
    let clamped = diff.clamp(-box_col.half_size, box_col.half_size);
    let closest = box_col.position + clamped;
    return (closest - circle_col.position).length() < circle_col.radius;
}

pub fn check_ray_box_intersection(ray: &Ray, box_col: &BoxCollider) -> Option<RayCollision> {
    let invdir = 1.0 / ray.direction;

    let mut t_near = (box_col.min() - ray.origin) * invdir;
    let mut t_far = (box_col.max() - ray.origin) * invdir;

    if t_far.y.is_nan() || t_far.x.is_nan() { return None; }
    if t_near.y.is_nan() || t_near.x.is_nan() { return None; }


    // sort the near and far on each axis
    if t_near.x > t_far.x { swap(&mut t_near.x, &mut t_far.x); }
    if t_near.y > t_far.y { swap(&mut t_near.y, &mut t_far.y); }

    if t_near.x > t_far.y || t_near.y > t_far.x { return None; }

    let t_hit_near = t_near.x.max(t_near.y);
    let t_hit_far = t_far.x.min(t_far.y);

    if t_hit_far < 0.0 { return None; };

    let contact_point = ray.origin + t_hit_near * ray.direction;

    let contact_normal = if t_near.x > t_near.y {
        if ray.direction.x < 0.0 {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(-1.0, 0.0)
        }
    } else if t_near.x < t_near.y {
        if ray.direction.y < 0.0 {
            Vec2::new(0.0, 1.0)
        } else {
            Vec2::new(0.0, -1.0)
        }
    } else {
        Vec2::ZERO
    };

    return Some(RayCollision{
        contact_point: contact_point,
        contact_normal: contact_normal,
        t: t_hit_near,
    });
}

pub enum Collider {
    Box(BoxCollider),
    Circle(CircleCollider)
}

impl Default for Collider {
    fn default() -> Self {
        Collider::Box(BoxCollider::default())
    }
}

fn debug_aabb(
    mut canvas: ResMut<Canvas>,
    collider_query: Query<(&Position, &Collider)>,
) {
    for (position, collider) in collider_query.iter() {
        match collider {
            Collider::Box(box_collider) => {
                canvas.draw(&Rectangle {
                    origin: position.0 + box_collider.position,
                    extents: box_collider.half_size * 2.0,
                    anchor_point: RectangleAnchor::Center
                }, DrawMode::stroke_1px(), Color::RED);
            },
            Collider::Circle(circle_collider) => {
                canvas.draw(&Circle {
                    center: position.0 + circle_collider.position,
                    radius: circle_collider.radius,
                }, DrawMode::stroke_1px(), Color::RED);
            },
        }
    }
}

fn debug_ray(
    windows: Res<Windows>,
    mut canvas: ResMut<Canvas>,
    q_camera: Query<&Transform, With<MainCamera>>,
    box_collide_query: Query<(&Collider, &Position)>
) {
    let wnd = windows.get_primary().unwrap();

    if let Some(pos) = wnd.cursor_position() {
        let size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
        let p = pos - size / 2.0;
        let camera_transform = q_camera.single().unwrap();

        let pos_wld = camera_transform.compute_matrix() * p.extend(0.0).extend(1.0);
        let origin = Vec2::new(-200.0, 200.0);
        let line = Line(origin, pos_wld.xy());
        canvas.draw(&line, DrawMode::stroke_1px(), Color::YELLOW);


        for (collider, pos) in box_collide_query.iter() {
            match collider {
                Collider::Box(box_collider) => {
                    if let Some(ray_collision) = check_ray_box_intersection(&Ray::new_from_points(origin, pos_wld.xy()), &box_collider.adjusted_position(&pos.0)) {
                        // println!("Ray collision: {}, {:?}, {:?}", ray_collision.t, ray_collision.contact_point, ray_collision.contact_normal);
                        if ray_collision.t <= 1.0 {
                            canvas.draw(&Circle {
                                center: ray_collision.contact_point,
                                radius: 5.0
                            }, DrawMode::fill_simple(), Color::RED);

                            canvas.draw(&Line(ray_collision.contact_point, ray_collision.contact_point + (ray_collision.contact_normal * 10.0)), DrawMode::stroke_1px(), Color::GREEN);
                        }
                    } else {
                    }
                },
                Collider::Circle(_) => { println!("Havent implemented circle ray"); },
            }
        }
    }
}

// Plugins

pub struct DebugCollidersPlugin;

impl Plugin for DebugCollidersPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(debug_aabb.system());
        app.add_system(debug_ray.system());
    }
}