use bevy::{input::common_conditions::input_toggle_active, prelude::*};
use bevy_inspector_egui::{bevy_egui::EguiContexts, egui::Slider};

use crate::car::{Car, VehicleConfig};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            config_ui_system.run_if(input_toggle_active(true, KeyCode::Escape)),
        );
    }
}

fn config_ui_system(
    mut contexts: EguiContexts,
    mut vehicle_configs: Query<&mut VehicleConfig, With<Car>>,
) {
    let mut vehicle_config = vehicle_configs.single_mut();
    bevy_inspector_egui::egui::Window::new("Settings").show(contexts.ctx_mut(), |ui| {
        ui.add(Slider::new(&mut vehicle_config.max_speed, 5.0..=200.0).text("max speed"));
        ui.add(Slider::new(&mut vehicle_config.max_force, 50.0..=1000.0).text("max force"));
    });
}
