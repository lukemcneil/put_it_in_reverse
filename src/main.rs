use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(
            0xF9 as f32 / 255.0,
            0xF9 as f32 / 255.0,
            0xFF as f32 / 255.0,
        )))
        .add_plugins((
            DefaultPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
        ))
        .add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
        ))
        .add_systems(Startup, setup_physics)
        .add_systems(
            Update,
            (
                calculate_tire_forces,
                sum_all_forces_on_car.after(calculate_tire_forces),
                draw_tire_force_gizmos.after(calculate_tire_forces),
                cast_ray,
                draw_tire_gizmos,
            ),
        )
        .register_type::<Car>()
        .register_type::<Tire>()
        .add_event::<AddForceToCar>()
        .run();
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Car;
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Tire {
    connected_to_engine: bool,
}

pub fn setup_physics(mut commands: Commands) {
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-30.0, 10.0, 0.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });

    // ground
    let ground_size = 200.1;
    let ground_height = 0.1;

    commands.spawn((
        TransformBundle::from(Transform::from_xyz(0.0, -ground_height, 0.0)),
        Collider::cuboid(ground_size, ground_height, ground_size),
        Friction::coefficient(0.7),
        Name::from("Floor"),
    ));

    // car and tires
    commands
        .spawn((
            TransformBundle::from(Transform::from_xyz(0., 10., 0.)),
            RigidBody::Dynamic,
            Collider::cuboid(3., 1., 1.),
            Car,
            Velocity::default(),
            ExternalForce::default(),
            Name::from("Car"),
            Friction::coefficient(0.7),
        ))
        .with_children(|child_builder| {
            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(3., -1.2, 1.5)),
                Tire {
                    connected_to_engine: true,
                },
                Name::from("Tire Front Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(3., -1.2, -1.5)),
                Tire {
                    connected_to_engine: true,
                },
                Name::from("Tire Front Left"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-3., -1.2, 1.5)),
                Tire {
                    connected_to_engine: false,
                },
                Name::from("Tire Back Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-3., -1.2, -1.5)),
                Tire {
                    connected_to_engine: false,
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

fn calculate_tire_forces(
    keys: Res<Input<KeyCode>>,
    car: Query<&Transform, With<Car>>,
    tires: Query<(&GlobalTransform, &Tire)>,
    mut ev_add_force_to_car: EventWriter<AddForceToCar>,
) {
    let car_transform = car.single();

    // handle acceleration and breaking forces
    for (tire_transform, tire) in &tires {
        let force_at_tire = car_transform.rotation.mul_vec3(Vec3::new(250.0, 0.0, 0.0));
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

    // handle turning forces (this will need to be changed)
    let turning_torque = car_transform.rotation.mul_vec3(Vec3::new(0.0, 0.0, 500.0));
    let mut torque_translation = car_transform.translation.clone();
    torque_translation += car_transform.rotation.mul_vec3(Vec3::new(3.0, 0.0, 0.0));
    if keys.pressed(KeyCode::D) {
        ev_add_force_to_car.send(AddForceToCar {
            force: turning_torque,
            point: torque_translation,
        });
    } else if keys.pressed(KeyCode::A) {
        ev_add_force_to_car.send(AddForceToCar {
            force: -turning_torque,
            point: torque_translation,
        });
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
    let scale_factor = 0.005;
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

fn cast_ray(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    rapier_context: Res<RapierContext>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    let window = windows.single();

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // We will color in read the colliders hovered by the mouse.
    for (camera, camera_transform) in &cameras {
        // First, compute a ray from the mouse position.
        let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            return;
        };

        // Then cast the ray.
        let hit = rapier_context.cast_ray(
            ray.origin,
            ray.direction,
            f32::MAX,
            true,
            QueryFilter::only_dynamic(),
        );

        if let Some((entity, _toi)) = hit {
            // Color in blue the entity we just hit.
            // Because of the query filter, only colliders attached to a dynamic body
            // will get an event.
            let color = Color::BLUE;
            commands.entity(entity).insert(ColliderDebugColor(color));
        }
    }
}
