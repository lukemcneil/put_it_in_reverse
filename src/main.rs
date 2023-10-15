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
        .add_systems(Update, (keyboard_input, cast_ray))
        .register_type::<Car>()
        .register_type::<Tire>()
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
                TransformBundle::from(Transform::from_xyz(3., -1., 1.)),
                Tire {
                    connected_to_engine: true,
                },
                Name::from("Tire Front Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(3., -1., -1.)),
                Tire {
                    connected_to_engine: true,
                },
                Name::from("Tire Front Left"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-3., -1., 1.)),
                Tire {
                    connected_to_engine: true,
                },
                Name::from("Tire Back Right"),
            ));

            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-3., -1., -1.)),
                Tire {
                    connected_to_engine: true,
                },
                Name::from("Tire Back Left"),
            ));
        });
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut car: Query<(&mut ExternalForce, &Transform, &Velocity), With<Car>>,
    tires: Query<(&GlobalTransform, &Tire), With<Tire>>,
) {
    let (mut external_force, car_transform, _velocity) = car.single_mut();
    // we need to calculate one final linear force and angular torque to apply to the car
    let mut final_force = ExternalForce::default();

    for (tire_transform, tire) in &tires {
        // handle acceleration and breaking forces
        let force_at_tire = car_transform.rotation.mul_vec3(Vec3::new(250.0, 0.0, 0.0));
        let tire_force_on_car = ExternalForce::at_point(
            force_at_tire,
            tire_transform.translation(),
            car_transform.translation,
        );
        if tire_transform.translation().y < 0.3 && tire.connected_to_engine {
            if keys.pressed(KeyCode::W) {
                final_force += tire_force_on_car;
            } else if keys.pressed(KeyCode::S) {
                final_force -= tire_force_on_car;
            }
        }

        // handle turning forces (this will need to be changed)
        if keys.pressed(KeyCode::D) {
            final_force.torque -= Vec3::new(0.0, 500.0, 0.0);
        } else if keys.pressed(KeyCode::A) {
            final_force.torque += Vec3::new(0.0, 500.0, 0.0);
        }
    }

    // set the external forces on the car to the calculated final_force
    external_force.force = final_force.force;
    external_force.torque = final_force.torque;
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
