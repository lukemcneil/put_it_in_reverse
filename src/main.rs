mod car;

use std::f32::consts::PI;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;
use car::{CameraPosition, Car};

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
        .add_systems(Update, camera_follow_car)
        .run();
}

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
    let mut ramp_tranform = Transform::from_xyz(50.0, -5.0, 0.0);
    ramp_tranform.rotate_z(PI / 4.0);
    commands.spawn((
        Collider::cuboid(20.0, 20.0, 20.0),
        Name::from("Ramp"),
        MaterialMeshBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 20.0 * 2.0 })),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(texture_handle.clone()),
                unlit: true,
                ..default()
            }),
            transform: ramp_tranform,
            global_transform: default(),
            ..default()
        },
    ));

    // car and trailer
    let car_entity = car::spawn_car(&mut commands);
    let trailer_entity = car::spawn_trailer(&mut commands);

    let joint = SphericalJointBuilder::new()
        .local_anchor1(Vec3::new(-3.5, 0.0, 0.0))
        .local_anchor2(Vec3::new(2.5, 0.0, 0.0));
    commands
        .get_entity(trailer_entity)
        .unwrap()
        .insert(ImpulseJoint::new(car_entity, joint));

    // add boxes to run into
    let mut box_parent_entity = commands.spawn((SpatialBundle::default(), Name::from("Obstacles")));
    let w = 10;
    let h = 5;
    for x in (-w / 2)..(w / 2) {
        for y in 0..h {
            box_parent_entity.with_children(|child_builder| {
                child_builder.spawn((
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
            });
        }
    }
}

fn camera_follow_car(
    mut camera: Query<&mut Transform, With<CarCamera>>,
    car_camera_desired_position: Query<&GlobalTransform, With<CameraPosition>>,
    car: Query<&GlobalTransform, With<Car>>,
    time: Res<Time>,
) {
    let new_cam_location = car_camera_desired_position.single();
    let mut car_camera = camera.single_mut();
    let lerped_position = car_camera
        .translation
        .lerp(new_cam_location.translation(), time.delta_seconds());
    car_camera.translation = Vec3::new(lerped_position.x, 30.0, lerped_position.z);
    car_camera.rotation = car_camera
        .looking_at(car.single().translation(), Vec3::Y)
        .rotation;
}
