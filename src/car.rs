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
                    suspension_force_calculations,
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
            ReadMassProperties::default(),
            Velocity::default(),
            ExternalForce::default(),
            Name::from("Car"),
            Friction::coefficient(0.5),
        ))
        .with_children(|child_builder| {
            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(2.5, -0.125, 1.0)),
                Tire {
                    connected_to_engine: true,
                    location: Location::Front,
                },
                Name::from("Tire Front Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(2.5, -0.125, -1.0)),
                Tire {
                    connected_to_engine: true,
                    location: Location::Front,
                },
                Name::from("Tire Front Left"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-2.5, -0.125, 1.0)),
                Tire {
                    connected_to_engine: false,
                    location: Location::Back,
                },
                Name::from("Tire Back Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-2.5, -0.125, -1.0)),
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

fn suspension_force_calculations(
    tires: Query<&GlobalTransform, With<Tire>>,
    car: Query<(&Velocity, &Transform), With<Car>>,
    rapier_context: Res<RapierContext>,
    mut ev_add_force_to_car: EventWriter<AddForceToCar>,
) {
    let (car_velocity, car_transform) = car.single();
    for tire_transform in &tires {
        let hit = rapier_context.cast_ray(
            tire_transform.translation(),
            tire_transform.down(),
            0.5,
            false,
            QueryFilter::only_fixed(),
        );
        if hit.is_some() {
            let spring_direction = tire_transform.up();
            let tire_velocity = car_velocity
                .linear_velocity_at_point(tire_transform.translation(), car_transform.translation);
            let offset = 0.5 - hit.unwrap().1;
            let velocity = spring_direction.dot(tire_velocity);
            let force = (offset * 10.0) - (velocity * 1.5);
            ev_add_force_to_car.send(AddForceToCar {
                force: spring_direction * force,
                point: tire_transform.translation(),
            });
        }
    }
}

fn lookup_power(velocity: Velocity) -> f32 {
    let max_speed = 30.0;
    let max_force: f32 = 100.0;
    let speed_ratio = velocity.linvel.length() / max_speed;
    let graph1 = -(-0.5 * speed_ratio + 0.3).log(10.0);
    let graph2 = 1.0;
    let graph3 = (-5.0 * speed_ratio + 6.0).log(10.0) + 0.6;
    let mut returned_force = 0.0;
    if speed_ratio < 0.0 {
        returned_force = 0.5 * max_force;
    } else if speed_ratio >= 0.0 && speed_ratio < 0.4 {
        returned_force = graph1 * max_force;
    } else if speed_ratio >= 0.4 && speed_ratio <= 0.698 {
        returned_force = graph2 * max_force;
    } else if speed_ratio > 0.698 && speed_ratio <= 1.0 {
        returned_force = graph3 * max_force;
    } else {
        return returned_force;
    }
    return returned_force;
}

fn calculate_tire_acceleration_and_braking_forces(
    keys: Res<Input<KeyCode>>,
    tires: Query<(&GlobalTransform, &Tire)>,
    car: Query<&Velocity, With<Car>>,
    mut ev_add_force_to_car: EventWriter<AddForceToCar>,
) {
    for (tire_transform, tire) in &tires {
        let force_at_tire = tire_transform
            .compute_transform()
            .rotation
            .mul_vec3(Vec3::new(lookup_power(*car.single()), 0.0, 0.0));
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
    let turning_radius = 0.296706;
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
    car: Query<(&Transform, &Velocity, &ReadMassProperties), With<Car>>,
    tires: Query<&GlobalTransform, With<Tire>>,
    mut ev_add_force_to_car: EventWriter<AddForceToCar>,
) {
    let tire_grip_strength = 0.7;
    let (car_transform, car_velocity, ReadMassProperties(car_mass)) = car.single();
    for tire_transform in &tires {
        if tire_transform.compute_transform().translation.y < 0.2 {
            let steering_direction = tire_transform.compute_transform().forward();
            let tire_velocity = car_velocity
                .linear_velocity_at_point(tire_transform.translation(), car_transform.translation);
            let steering_velocity = steering_direction.dot(tire_velocity);
            let desired_velocity_change = -steering_velocity * tire_grip_strength;
            let desired_acceleration = desired_velocity_change * 5.0;
            ev_add_force_to_car.send(AddForceToCar {
                force: steering_direction * desired_acceleration * car_mass.mass,
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
