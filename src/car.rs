use bevy::{prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::*;

pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AddForce>()
            .add_systems(
                Update,
                (
                    turn_tires,
                    calculate_tire_acceleration_and_braking_forces,
                    calculate_tire_turning_forces,
                    calculate_tire_suspension_forces,
                    (sum_all_forces, draw_tire_force_gizmos)
                        .after(calculate_tire_acceleration_and_braking_forces)
                        .after(calculate_tire_turning_forces)
                        .after(calculate_tire_suspension_forces),
                    draw_tire_gizmos,
                ),
            )
            .register_type::<Car>()
            .register_type::<Drivable>()
            .register_type::<Tire>()
            .register_type::<CameraPosition>();
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct Car;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Drivable;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Tire {
    connected_to_engine: bool,
    location: TireLocation,
}

#[derive(Default, Reflect)]
enum TireLocation {
    #[default]
    Front,
    Back,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct CameraPosition;

pub fn spawn_car(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            TransformBundle::from(Transform::from_xyz(0., 10., 0.)),
            RigidBody::Dynamic,
            Collider::cuboid(3., 0.25, 1.),
            Car,
            Drivable,
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
                    location: TireLocation::Front,
                },
                Name::from("Tire Front Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(2.5, -0.125, -1.0)),
                Tire {
                    connected_to_engine: true,
                    location: TireLocation::Front,
                },
                Name::from("Tire Front Left"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-2.5, -0.125, 1.0)),
                Tire {
                    connected_to_engine: false,
                    location: TireLocation::Back,
                },
                Name::from("Tire Back Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-2.5, -0.125, -1.0)),
                Tire {
                    connected_to_engine: false,
                    location: TireLocation::Back,
                },
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

pub fn spawn_trailer(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            TransformBundle::from(Transform::from_xyz(-5.0, 10.0, 0.0)),
            RigidBody::Dynamic,
            Collider::cuboid(2.0, 0.25, 2.0),
            Friction::coefficient(0.5),
            Drivable,
            Name::from("Trailer"),
            Velocity::default(),
            ReadMassProperties::default(),
            ExternalForce::default(),
        ))
        .with_children(|child_builder| {
            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-0.5, -0.25, 2.1)),
                Tire {
                    connected_to_engine: false,
                    location: TireLocation::Back,
                },
                Name::from("Tire Trailer Right"),
            ));
            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-0.5, -0.25, -2.1)),
                Tire {
                    connected_to_engine: false,
                    location: TireLocation::Back,
                },
                Name::from("Tire Trailer Left"),
            ));
        })
        .id()
}

#[derive(Event)]
struct AddForce {
    force: Vec3,
    point: Vec3,
    entity: Entity,
}

fn calculate_tire_suspension_forces(
    tires: Query<(&GlobalTransform, &Parent), With<Tire>>,
    drivables: Query<(Entity, &Velocity, &Transform), With<Drivable>>,
    rapier_context: Res<RapierContext>,
    mut add_forces: EventWriter<AddForce>,
) {
    for (tire_transform, parent) in &tires {
        let (parent_entity, parent_velocity, parent_transform) =
            drivables.get(parent.get()).unwrap();
        let hit = rapier_context.cast_ray(
            tire_transform.translation(),
            tire_transform.down(),
            1.5,
            false,
            QueryFilter::only_fixed(),
        );
        if let Some((_, hit_distance)) = hit {
            let spring_direction = tire_transform.up();
            let tire_velocity = parent_velocity.linear_velocity_at_point(
                tire_transform.translation(),
                parent_transform.translation,
            );
            let offset = 1.5 - hit_distance;
            let velocity = spring_direction.dot(tire_velocity);
            let force = (offset * 100.0) - (velocity * 10.0);
            add_forces.send(AddForce {
                force: spring_direction * force,
                point: tire_transform.translation(),
                entity: parent_entity,
            });
        }
    }
}

fn lookup_power(velocity: Velocity) -> f32 {
    let max_speed = 20.0;
    let max_force: f32 = 50.0;
    let speed_ratio = velocity.linvel.length() / max_speed;
    let lookup = if speed_ratio < 0.0 {
        0.5
    } else if speed_ratio >= 0.0 && speed_ratio < 0.4 {
        -(-0.5 * speed_ratio + 0.3).log(10.0)
    } else if speed_ratio >= 0.4 && speed_ratio <= 0.698 {
        1.0
    } else if speed_ratio > 0.698 && speed_ratio <= 1.0 {
        (-5.0 * speed_ratio + 6.0).log(10.0) + 0.6
    } else {
        0.0
    };
    max_force * lookup
}

fn calculate_tire_acceleration_and_braking_forces(
    keys: Res<Input<KeyCode>>,
    tires: Query<(&GlobalTransform, &Parent, &Tire)>,
    car: Query<(Entity, &Velocity), With<Drivable>>,
    mut add_forces: EventWriter<AddForce>,
) {
    for (tire_transform, parent, tire) in &tires {
        let (parent_entity, parent_velocity) = car.get(parent.get()).unwrap();
        let force_at_tire = tire_transform
            .compute_transform()
            .rotation
            .mul_vec3(Vec3::new(lookup_power(*parent_velocity), 0.0, 0.0));
        if tire_transform.translation().y < 2.0 && tire.connected_to_engine {
            if keys.pressed(KeyCode::W) {
                add_forces.send(AddForce {
                    force: force_at_tire,
                    point: tire_transform.translation(),
                    entity: parent_entity,
                });
            } else if keys.pressed(KeyCode::S) {
                add_forces.send(AddForce {
                    force: -force_at_tire,
                    point: tire_transform.translation(),
                    entity: parent_entity,
                });
            }
        }
    }
}

fn turn_tires(keys: Res<Input<KeyCode>>, mut tires: Query<(&mut Transform, &Tire)>) {
    let turning_radius = 0.296706;
    for (mut tire_transform, tire) in &mut tires {
        if let TireLocation::Front = tire.location {
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
    car: Query<(Entity, &Transform, &Velocity, &ReadMassProperties), With<Drivable>>,
    tires: Query<(&GlobalTransform, &Parent), With<Tire>>,
    mut add_forces: EventWriter<AddForce>,
) {
    let tire_grip_strength = 0.7;

    for (tire_transform, parent) in &tires {
        let (parent_entity, parent_transform, parent_velocity, ReadMassProperties(car_mass)) =
            car.get(parent.get()).unwrap();
        if tire_transform.compute_transform().translation.y < 2.0 {
            let steering_direction = tire_transform.compute_transform().forward();
            let tire_velocity = parent_velocity.linear_velocity_at_point(
                tire_transform.translation(),
                parent_transform.translation,
            );
            let steering_velocity = steering_direction.dot(tire_velocity);
            let desired_velocity_change = -steering_velocity * tire_grip_strength;
            let desired_acceleration = desired_velocity_change * 60.0;
            add_forces.send(AddForce {
                force: steering_direction * desired_acceleration * (car_mass.mass / 4.0),
                point: tire_transform.translation(),
                entity: parent_entity,
            });
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
