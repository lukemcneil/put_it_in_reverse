use bevy::{prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::*;

use crate::{CameraPosition, Car, CarCamera, Drivable, Location, Tire};

pub struct CarPlugin;

#[derive(Resource, Default)]
pub struct VehicleConfigs {
    pub configs: HashMap<String, VehicleConfig>,
}

pub struct VehicleConfig {
    pub height: f32,
    pub width: f32,
    pub length: f32,
    pub wheelbase: f32,
    pub spring_offset: f32,
    pub spring_power: f32,
    pub shock: f32,
    pub max_speed: f32,
    pub max_force: f32,
    pub turn_radius: f32,
    pub weight: f32,
}

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AddForce>()
            .register_type::<Car>()
            .register_type::<Tire>()
            .insert_resource(VehicleConfigs {
                configs: HashMap::new(),
            })
            .add_systems(Startup, make_vehicles)
            .add_systems(
                Update,
                (
                    turn_tires,
                    calculate_tire_acceleration_and_braking_forces,
                    calculate_tire_turning_forces,
                    suspension_force_calculations,
                    camera_follow_car,
                    (sum_all_forces, draw_tire_force_gizmos)
                        .after(calculate_tire_acceleration_and_braking_forces)
                        .after(calculate_tire_turning_forces),
                    draw_tire_gizmos,
                ),
            );
    }
}

fn make_vehicles(mut vehicle_configs: ResMut<VehicleConfigs>) {
    vehicle_configs.configs.insert(
        "F150".into(),
        VehicleConfig {
            height: 1.452024 / 2.0,
            width: 2.02946 / 2.0,
            length: 5.31114 / 2.0,
            wheelbase: 3.11912 / 2.0,
            spring_offset: 1.252926,
            spring_power: 300.0,
            shock: 45.0,
            max_speed: 50.0,
            max_force: 100.0,
            turn_radius: 0.45811518324607,
            weight: 30.0,
        },
    );
}

pub fn spawn_car(commands: &mut Commands, vehicle_configs: Res<VehicleConfigs>) -> Entity {
    let vehicle_config = vehicle_configs.configs.get("F150").unwrap();
    commands
        .spawn((
            TransformBundle::from(Transform::from_xyz(0., vehicle_config.height, 0.)),
            RigidBody::Dynamic,
            Collider::cuboid(
                vehicle_config.length,
                vehicle_config.height,
                vehicle_config.width,
            ),
            AdditionalMassProperties::Mass(vehicle_config.weight - 26.0),
            Car,
            Drivable,
            Velocity::default(),
            ExternalForce::default(),
            Name::from("Car"),
            Friction::coefficient(1.0),
        ))
        .with_children(|child_builder| {
            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(
                    vehicle_config.wheelbase,
                    -vehicle_config.height / 3.0,
                    vehicle_config.width,
                )),
                Tire {
                    connected_to_engine: true,
                    location: Location::Front,
                },
                AdditionalMassProperties::Mass(5.0),
                Name::from("Tire Front Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(
                    vehicle_config.wheelbase,
                    -vehicle_config.height / 3.0,
                    -vehicle_config.width,
                )),
                Tire {
                    connected_to_engine: true,
                    location: Location::Front,
                },
                AdditionalMassProperties::Mass(5.0),
                Name::from("Tire Front Left"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(
                    -vehicle_config.wheelbase,
                    -vehicle_config.height / 3.0,
                    vehicle_config.width,
                )),
                Tire {
                    connected_to_engine: false,
                    location: Location::Back,
                },
                AdditionalMassProperties::Mass(5.0),
                Name::from("Tire Back Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(
                    -vehicle_config.wheelbase,
                    -vehicle_config.height / 3.0,
                    -vehicle_config.width,
                )),
                Tire {
                    connected_to_engine: false,
                    location: Location::Back,
                },
                AdditionalMassProperties::Mass(5.0),
                Name::from("Tire Back Left"),
            ));

            child_builder.spawn((
                TransformBundle::from(
                    Transform::from_xyz(-40.0, 40.0, 0.0)
                        .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
                ),
                CameraPosition,
                Name::from("Camera Desired Position"),
            ));
        })
        .id()
}

fn camera_follow_car(
    mut camera: Query<&mut Transform, With<CarCamera>>,
    car_camera_desired_position: Query<&GlobalTransform, With<CameraPosition>>,
    car: Query<&GlobalTransform, With<Car>>,
) {
    let new_cam_location = car_camera_desired_position.single();
    let mut car_camera = camera.single_mut();
    let lerped_position = car_camera
        .translation
        .lerp(new_cam_location.translation(), 0.01);
    car_camera.translation = Vec3::new(lerped_position.x, 30.0, lerped_position.z);
    car_camera.rotation = car_camera
        .looking_at(car.single().translation(), Vec3::Y)
        .rotation;
}

#[derive(Event)]
struct AddForce {
    force: Vec3,
    point: Vec3,
    entity: Entity,
}

fn suspension_force_calculations(
    tires: Query<(&GlobalTransform, &Parent), With<Tire>>,
    car: Query<(Entity, &Velocity, &Transform), With<Drivable>>,
    rapier_context: Res<RapierContext>,
    vehicle_configs: Res<VehicleConfigs>,
    mut add_forces: EventWriter<AddForce>,
) {
    let vehicle_config = vehicle_configs.configs.get("F150").unwrap();
    for (tire_transform, parent) in &tires {
        let (parent_entity, parent_velocity, parent_transform) = car.get(parent.get()).unwrap();
        let hit = rapier_context.cast_ray(
            tire_transform.translation(),
            tire_transform.down(),
            1.5,
            false,
            QueryFilter::only_fixed(),
        );
        if hit.is_some() {
            let spring_direction = tire_transform.up();
            let tire_velocity = parent_velocity.linear_velocity_at_point(
                tire_transform.translation(),
                parent_transform.translation,
            );
            let offset = vehicle_config.spring_offset - hit.unwrap().1;
            let velocity = spring_direction.dot(tire_velocity);
            let force = (offset * vehicle_config.spring_power) - (velocity * vehicle_config.shock);
            add_forces.send(AddForce {
                force: spring_direction * force,
                point: tire_transform.translation(),
                entity: parent_entity,
            });
        }
    }
}

fn lookup_power(velocity: Velocity, max_speed: f32, max_force: f32) -> f32 {
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
    tires: Query<(&GlobalTransform, &Parent, &Tire)>,
    car: Query<(Entity, &Velocity, &Transform), With<Drivable>>,
    vehicle_configs: Res<VehicleConfigs>,
    mut add_forces: EventWriter<AddForce>,
    rapier_context: Res<RapierContext>,
) {
    let vehicle_config = vehicle_configs.configs.get("F150").unwrap();
    let coefficient_of_friction = 0.012;
    for (tire_transform, parent, tire) in &tires {
        let (parent_entity, parent_velocity, parent_transform) = car.get(parent.get()).unwrap();
        let force_at_tire = tire_transform
            .compute_transform()
            .rotation
            .mul_vec3(Vec3::new(
                lookup_power(
                    *parent_velocity,
                    vehicle_config.max_speed,
                    vehicle_config.max_force,
                ),
                0.0,
                0.0,
            ));
        let hit = rapier_context.cast_ray(
            tire_transform.translation(),
            tire_transform.down(),
            1.5,
            false,
            QueryFilter::only_fixed(),
        );
        if hit.is_some() {
            if hit.unwrap().1 < vehicle_config.spring_offset * 1.25 && tire.connected_to_engine {
                if keys.pressed(KeyCode::W) {
                    add_forces.send(AddForce {
                        force: force_at_tire,
                        point: tire_transform.translation(),
                        entity: parent_entity,
                    });
                } else if keys.pressed(KeyCode::S) {
                    add_forces.send(AddForce {
                        force: -force_at_tire / 3.0,
                        point: tire_transform.translation(),
                        entity: parent_entity,
                    });
                } else if parent_velocity.linvel.x.abs() > 0.0
                    || parent_velocity.linvel.z.abs() > 0.0
                {
                    let mut negative_check = -1.0;
                    if parent_velocity.linvel.dot(parent_transform.right()) < 0.0 {
                        negative_check = 1.0;
                    }
                    add_forces.send(AddForce {
                        force: negative_check
                            * tire_transform
                                .compute_transform()
                                .rotation
                                .mul_vec3(Vec3::new(
                                    (vehicle_config.weight / 4.0) * coefficient_of_friction * 9.81,
                                    0.0,
                                    0.0,
                                )),
                        point: tire_transform.translation(),
                        entity: parent_entity,
                    });
                }
            }
        }
    }
}

fn turn_tires(
    keys: Res<Input<KeyCode>>,
    mut tires: Query<(&mut Transform, &Tire)>,
    vehicle_configs: Res<VehicleConfigs>,
) {
    let vehicle_config = vehicle_configs.configs.get("F150").unwrap();
    let turning_radius = vehicle_config.turn_radius;
    for (mut tire_transform, tire) in &mut tires {
        if let Location::Front = tire.location {
            if keys.pressed(KeyCode::D) {
                tire_transform.rotation = tire_transform
                    .rotation
                    .lerp(Quat::from_axis_angle(Vec3::Y, -turning_radius), 0.002);
            } else if keys.pressed(KeyCode::A) {
                tire_transform.rotation = tire_transform
                    .rotation
                    .lerp(Quat::from_axis_angle(Vec3::Y, turning_radius), 0.002);
            } else {
                tire_transform.rotation = tire_transform.rotation.lerp(Quat::IDENTITY, 0.1);
            }
        }
    }
}

fn calculate_tire_turning_forces(
    car: Query<(Entity, &Transform, &Velocity), With<Drivable>>,
    tires: Query<(&GlobalTransform, &Parent), With<Tire>>,
    mut add_forces: EventWriter<AddForce>,
    vehicle_configs: Res<VehicleConfigs>,
    rapier_context: Res<RapierContext>,
) {
    let vehicle_config = vehicle_configs.configs.get("F150").unwrap();
    let tire_grip_strength = 0.7;
    for (tire_transform, parent) in &tires {
        let (parent_entity, parent_transform, parent_velocity) = car.get(parent.get()).unwrap();
        let hit = rapier_context.cast_ray(
            tire_transform.translation(),
            tire_transform.down(),
            1.5,
            false,
            QueryFilter::only_fixed(),
        );
        if hit.is_some() {
            if hit.unwrap().1 < vehicle_config.spring_offset * 1.25 {
                let steering_direction = tire_transform.compute_transform().forward();
                let tire_velocity = parent_velocity.linear_velocity_at_point(
                    tire_transform.translation(),
                    parent_transform.translation,
                );
                let steering_velocity = steering_direction.dot(tire_velocity);
                let desired_velocity_change = -steering_velocity * tire_grip_strength;
                let desired_acceleration = desired_velocity_change * 60.0;
                add_forces.send(AddForce {
                    force: steering_direction
                        * desired_acceleration
                        * (vehicle_config.weight / 4.0),
                    point: tire_transform.translation(),
                    entity: parent_entity,
                });
            }
        }
    }
}

fn sum_all_forces(
    mut add_forces: EventReader<AddForce>,
    mut drivables: Query<(Entity, &Transform, &mut ExternalForce), With<Drivable>>,
) {
    let mut final_forces = HashMap::new();
    for (entity, _, _) in &drivables {
        final_forces.insert(entity, ExternalForce::default());
    }

    for AddForce {
        force,
        point,
        entity,
    } in add_forces.iter()
    {
        let (_, transform, _) = drivables.get(entity.clone()).unwrap();
        let force_on_entity = ExternalForce::at_point(*force, *point, transform.translation);
        *final_forces.get_mut(entity).unwrap() += force_on_entity;
    }

    // set the external forces on the entity to the calculated final_force
    for (entity, _, mut external_force) in &mut drivables {
        let final_force = final_forces.get(&entity).unwrap();
        external_force.force = final_force.force;
        external_force.torque = final_force.torque;
    }
}

fn draw_tire_force_gizmos(mut add_forces: EventReader<AddForce>, mut gizmos: Gizmos) {
    let scale_factor = 0.04;
    for AddForce {
        force,
        point,
        entity: _,
    } in add_forces.iter()
    {
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
