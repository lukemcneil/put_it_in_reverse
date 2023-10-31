use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::car::Trailer;

pub struct ParkingSpotPlugin;

impl Plugin for ParkingSpotPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ParkingSpot>()
            .insert_resource(ParkingSpotInfo {
                transform: Transform::from_scale(Vec3 {
                    x: 10.0,
                    y: 1.5,
                    z: 3.0,
                })
                .with_translation(Vec3 {
                    x: 0.0,
                    y: -8.0,
                    z: -5.0,
                }),
                trailer_in: false,
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
    trailer_in: bool,
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
        if parking_spot_info.trailer_in {
            Color::GREEN
        } else {
            Color::RED
        },
    );
}

fn check_if_trailer_in_parking_spot(
    mut collision_events: EventReader<CollisionEvent>,
    trailer: Query<&Trailer>,
    mut parking_spot_status: ResMut<ParkingSpotInfo>,
) {
    for collision_event in collision_events.iter() {
        match collision_event {
            CollisionEvent::Started(_, collided_entity, _) => {
                if let Ok(_) = trailer.get(*collided_entity) {
                    parking_spot_status.trailer_in = true;
                }
            }
            CollisionEvent::Stopped(_, left_entity, _) => {
                if let Ok(_) = trailer.get(*left_entity) {
                    parking_spot_status.trailer_in = false;
                }
            }
        }
    }
}
