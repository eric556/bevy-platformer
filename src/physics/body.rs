use bevy::{math::{IVec2, Vec2}, prelude::Bundle};

#[derive(Default, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

#[derive(Default, Debug)]
pub struct Velocity(pub Vec2);

#[derive(Default, Debug)]
pub struct Acceleration(pub Vec2, pub Vec2);

#[derive(Default, Debug)]
pub struct Remainder(pub Vec2);

#[derive(PartialEq, Debug)]
pub enum BodyType {
    Actor,
    Solid
}

impl Default for BodyType {
    fn default() -> Self {
        BodyType::Solid
    }
}

#[derive(Bundle, Default, Debug)]
pub struct BodyBundle {
    pub body_type: BodyType,
    pub velocity: Velocity,
    pub position: Position,
    pub remainder: Remainder,
}