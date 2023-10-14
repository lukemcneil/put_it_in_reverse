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
        .add_systems(Startup, (setup_graphics, setup_physics))
        .add_systems(Update, (keyboard_input, cast_ray))
        .run();
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10.0, 10.0, 30.0)
            .looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

#[derive(Component)]
struct Car;

pub fn setup_physics(mut commands: Commands) {
    /*
     * Ground
     */
    let ground_size = 200.1;
    let ground_height = 0.1;

    commands.spawn((
        TransformBundle::from(Transform::from_xyz(0.0, -ground_height, 0.0)),
        Collider::cuboid(ground_size, ground_height, ground_size),
    ));

    /*
     * Create the car
     */
    commands.spawn((
        TransformBundle::from(Transform::from_xyz(0., 10., 0.)),
        RigidBody::Dynamic,
        Collider::cuboid(3., 1., 1.),
        Car,
        ExternalForce::default(),
    ));
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut car: Query<(&mut ExternalForce, &Transform), With<Car>>,
) {
    let (mut external_force, transform) = car.single_mut();

    if keys.pressed(KeyCode::W) {
        external_force.force = transform.rotation.mul_vec3(Vec3::new(1000.0, 0.0, 0.0));
    } else if keys.pressed(KeyCode::S) {
        external_force.force = transform.rotation.mul_vec3(Vec3::new(-1000.0, 0.0, 0.0));
    } else {
        external_force.force = Vec3::ZERO;
    }

    if keys.pressed(KeyCode::D) {
        external_force.torque = Vec3::new(0.0, -1000.0, 0.0);
    } else if keys.pressed(KeyCode::A) {
        external_force.torque = Vec3::new(0.0, 1000.0, 0.0);
    } else {
        external_force.torque = Vec3::ZERO;
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
