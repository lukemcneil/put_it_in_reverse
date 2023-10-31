use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::car::VehicleCornerCollider;

pub struct ParkingSpotPlugin;

impl Plugin for ParkingSpotPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ParkingSpot>()
            .insert_resource(ParkingSpotInfo {
                transform: Transform::from_scale(Vec3 {
                    x: 10.0,
                    y: 1.5,
                    z: 4.0,
                })
                .with_translation(Vec3 {
                    x: 0.0,
                    y: -8.0,
                    z: -0.0,
                }),
                trailer_tires_in: 0,
            })
            .add_systems(Startup, spawn_parking_spot)
            .add_systems(
                Update,
                (draw_parking_spot, check_if_trailer_in_parking_spot),
            );
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct ParkingSpot;

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct ParkingSpotInfo {
    transform: Transform,
    trailer_tires_in: i32,
}

fn spawn_parking_spot(mut commands: Commands, parking_spot_info: Res<ParkingSpotInfo>) {
    commands.spawn((
        TransformBundle {
            local: parking_spot_info.transform,
            ..default()
        },
        Collider::cuboid(0.5, 0.5, 0.5),
        Sensor,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

fn draw_parking_spot(mut gizmos: Gizmos, parking_spot_info: Res<ParkingSpotInfo>) {
    gizmos.cuboid(
        parking_spot_info.transform,
        if parking_spot_info.trailer_tires_in == 4 {
            Color::GREEN
        } else {
            Color::RED
        },
    );
}

fn check_if_trailer_in_parking_spot(
    mut collision_events: EventReader<CollisionEvent>,
    tire_colliders: Query<&VehicleCornerCollider>,
    mut parking_spot_status: ResMut<ParkingSpotInfo>,
) {
    for collision_event in collision_events.iter() {
        match collision_event {
            CollisionEvent::Started(_, collided_entity, _) => {
                if let Ok(tire_collider) = tire_colliders.get(*collided_entity) {
                    if !tire_collider.is_car {
                        parking_spot_status.trailer_tires_in += 1;
                    }
                }
            }
            CollisionEvent::Stopped(_, left_entity, _) => {
                if let Ok(tire_collider) = tire_colliders.get(*left_entity) {
                    if !tire_collider.is_car {
                        parking_spot_status.trailer_tires_in -= 1;
                    }
                }
            }
        }
    }
}
