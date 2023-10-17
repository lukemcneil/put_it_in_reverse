mod car;

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
        .add_plugins(car::CarPlugin)
        .add_systems(Startup, setup_physics)
        .add_systems(Update, cast_ray)
        .run();
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum Location {
    #[default]
    Front,
    Back,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Car;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Drivable;
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Tire {
    connected_to_engine: bool,
    location: Location,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct CameraPosition;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct CarCamera;

pub fn setup_physics(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-50.0, 50.0, 0.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        },
        CarCamera,
    ));

    // ground
    let ground_size = 100.0;
    let ground_height = 0.1;

    let texture_handle = asset_server.load("floor.png");
    commands.spawn((
        Collider::cuboid(ground_size, ground_height, ground_size),
        Name::from("Floor"),
        MaterialMeshBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: ground_size * 2.0,
                subdivisions: 0,
            })),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(texture_handle.clone()),
                // TODO: remove this unlit, then add a sun and headlights
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, -ground_height, 0.0),
            global_transform: default(),
            ..default()
        },
    ));

    // car and tires
    let car_entity = car::spawn_car(&mut commands);
    let joint = SphericalJointBuilder::new()
        .local_anchor1(Vec3::new(-3.5, 0.0, 0.0))
        .local_anchor2(Vec3::new(2.5, 0.0, 0.0));
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
                    location: Location::Back,
                },
                Name::from("Tire Trailer Right"),
            ));
            child_builder.spawn((
                TransformBundle::from(Transform::from_xyz(-0.5, -0.25, -2.1)),
                Tire {
                    connected_to_engine: false,
                    location: Location::Back,
                },
                Name::from("Tire Trailer Left"),
            ));
        })
        .insert(ImpulseJoint::new(car_entity, joint));

    // add boxes to run into
    let w = 10;
    let h = 5;
    for x in (-w / 2)..(w / 2) {
        for y in 0..h {
            commands.spawn((
                TransformBundle::from(Transform::from_xyz(
                    10.0,
                    (y as f32) * 1.5,
                    (x as f32) * 1.5,
                )),
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                Friction::coefficient(0.5),
                Name::from(format!("Box ({},{})", x, y)),
            ));
        }
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
