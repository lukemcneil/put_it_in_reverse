use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{Car, Location, Tire};

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AddForceToCar>()
            .register_type::<Car>()
            .register_type::<Tire>()
            .add_systems(
                Update,
                (
                    turn_tires,
                    calculate_tire_acceleration_and_braking_forces,
                    calculate_tire_turning_forces,
                    (sum_all_forces_on_car, draw_tire_force_gizmos)
                        .after(calculate_tire_acceleration_and_braking_forces)
                        .after(calculate_tire_turning_forces),
                    draw_tire_gizmos,
                ),
            );
    }
}

pub fn spawn_car(commands: &mut Commands) {
    commands
        .spawn((
            TransformBundle::from(Transform::from_xyz(0., 10., 0.)),
            RigidBody::Dynamic,
            Collider::cuboid(3., 0.25, 1.),
            Car,
            Velocity::default(),
            ExternalForce::default(),
            Name::from("Car"),
            Friction::coefficient(0.7),
        ))
        .with_children(|child_builder| {
            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(3., -0.25, 1.7)),
                Tire {
                    connected_to_engine: true,
                    location: Location::Front,
                },
                Name::from("Tire Front Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(3., -0.25, -1.7)),
                Tire {
                    connected_to_engine: true,
                    location: Location::Front,
                },
                Name::from("Tire Front Left"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-3., -0.25, 1.7)),
                Tire {
                    connected_to_engine: false,
                    location: Location::Back,
                },
                Name::from("Tire Back Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-3., -0.25, -1.7)),
                Tire {
                    connected_to_engine: false,
                    location: Location::Back,
                },
                Name::from("Tire Back Left"),
            ));
        });
}

#[derive(Event, Default)]
struct AddForceToCar {
    force: Vec3,
    point: Vec3,
}

fn calculate_tire_acceleration_and_braking_forces(
    keys: Res<Input<KeyCode>>,
    tires: Query<(&GlobalTransform, &Tire)>,
    mut ev_add_force_to_car: EventWriter<AddForceToCar>,
) {
    for (tire_transform, tire) in &tires {
        let force_at_tire = tire_transform
            .compute_transform()
            .rotation
            .mul_vec3(Vec3::new(75.0, 0.0, 0.0));
        if tire_transform.translation().y < 0.3 && tire.connected_to_engine {
            if keys.pressed(KeyCode::W) {
                ev_add_force_to_car.send(AddForceToCar {
                    force: force_at_tire,
                    point: tire_transform.translation(),
                });
            } else if keys.pressed(KeyCode::S) {
                ev_add_force_to_car.send(AddForceToCar {
                    force: -force_at_tire,
                    point: tire_transform.translation(),
                });
            }
        }
    }
}

fn turn_tires(keys: Res<Input<KeyCode>>, mut tires: Query<(&mut Transform, &Tire)>) {
    let turning_radius = PI / 5.0;
    for (mut tire_transform, tire) in &mut tires {
        if let Location::Front = tire.location {
            if keys.pressed(KeyCode::D) {
                tire_transform.rotation = Quat::from_axis_angle(Vec3::Y, -turning_radius);
            } else if keys.pressed(KeyCode::A) {
                tire_transform.rotation = Quat::from_axis_angle(Vec3::Y, turning_radius);
            } else {
                tire_transform.rotation = Quat::IDENTITY;
            }
        }
    }
}

fn calculate_tire_turning_forces(
    car: Query<(&Transform, &Velocity), With<Car>>,
    tires: Query<&GlobalTransform, With<Tire>>,
    mut ev_add_force_to_car: EventWriter<AddForceToCar>,
) {
    let tire_grip_strength = 0.7;
    for tire_transform in &tires {
        if tire_transform.compute_transform().translation.y < 0.2 {
            let car_transform = car.single();
            let mut torque_translation = car_transform.0.translation.clone();
            torque_translation += car_transform.0.rotation.mul_vec3(Vec3::new(3.0, 0.0, 0.0));
            let steering_direction = tire_transform.compute_transform().forward();
            let tire_velocity = car_transform.1.linear_velocity_at_point(
                tire_transform.translation(),
                car_transform.0.translation,
            );
            let steering_velocity = steering_direction.dot(tire_velocity);
            let desired_velocity_change = -steering_velocity * tire_grip_strength;
            let desired_acceleration = desired_velocity_change;
            ev_add_force_to_car.send(AddForceToCar {
                force: steering_direction * desired_acceleration * 10.0,
                point: tire_transform.translation(),
            });
        }
    }
}

fn sum_all_forces_on_car(
    mut ev_add_force_to_car: EventReader<AddForceToCar>,
    mut car: Query<(&mut ExternalForce, &Transform), With<Car>>,
) {
    // we need to calculate one final linear force and angular torque to apply to the car
    let mut final_force = ExternalForce::default();

    let (mut external_force, car_transform) = car.single_mut();
    for AddForceToCar { force, point } in ev_add_force_to_car.iter() {
        let force_on_car = ExternalForce::at_point(*force, *point, car_transform.translation);
        final_force += force_on_car;
    }

    // set the external forces on the car to the calculated final_force
    external_force.force = final_force.force;
    external_force.torque = final_force.torque;
}

fn draw_tire_force_gizmos(mut ev_add_force_to_car: EventReader<AddForceToCar>, mut gizmos: Gizmos) {
    let scale_factor = 0.04;
    for AddForceToCar { force, point } in ev_add_force_to_car.iter() {
        gizmos.ray(*point, *force * scale_factor, Color::BLUE);
    }
}

fn draw_tire_gizmos(mut gizmos: Gizmos, tires: Query<(&GlobalTransform, &Tire)>) {
    for (global_transform, tire) in &tires {
        gizmos.sphere(
            global_transform.translation(),
            Quat::IDENTITY,
            0.3,
            if tire.connected_to_engine {
                Color::RED
            } else {
                Color::BLACK
            },
        );
    }
}
