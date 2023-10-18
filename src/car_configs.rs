use bevy::prelude::*;

use crate::car::VehicleConfig;

const CAR_LENGTH: f32 = 5.31114 / 2.0;
pub const CAR_CONFIG: VehicleConfig = VehicleConfig {
    height: 1.452024 / 2.0,
    width: 2.02946 / 2.0,
    length: CAR_LENGTH,
    wheelbase: 3.11912 / 2.0,
    wheel_offset: 0.0,
    spring_offset: 1.252926,
    spring_power: 300.0,
    shock: 45.0,
    max_speed: 50.0,
    max_force: 100.0,
    turn_radius: 0.45811518324607,
    anchor_point: Vec3 {
        x: -CAR_LENGTH - 0.787,
        y: -0.7,
        z: 0.0,
    },
};

const TRAILER_LENGTH: f32 = 7.8768 / 2.0;
pub const TRAILER_CONFIG: VehicleConfig = VehicleConfig {
    height: 0.18234 / 2.0,
    width: 2.159 / 2.0,
    length: TRAILER_LENGTH,
    wheelbase: 1.0 / 2.0,
    wheel_offset: -1.0,
    spring_offset: 1.0,
    spring_power: 0.0,
    shock: 0.0,
    max_speed: 0.0,
    max_force: 0.0,
    turn_radius: 0.0,
    anchor_point: Vec3 {
        x: TRAILER_LENGTH + 0.787,
        y: -(0.18234 / 2.0),
        z: 0.0,
    },
};
