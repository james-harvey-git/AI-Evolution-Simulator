use egui;

use crate::simulation::SimState;
use super::UiState;

/// Top toolbar with simulation controls and panel toggles.
pub fn draw_toolbar(ctx: &egui::Context, sim: &mut SimState, ui_state: &mut UiState) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Pause/Play
            let pause_label = if sim.paused { "▶ Play" } else { "⏸ Pause" };
            if ui.button(pause_label).clicked() {
                sim.paused = !sim.paused;
            }

            ui.separator();

            // Speed control
            ui.label("Speed:");
            let speeds = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
            for &s in &speeds {
                let label = format!("{s}x");
                let selected = (sim.speed_multiplier - s).abs() < 0.01;
                if ui.selectable_label(selected, &label).clicked() {
                    sim.speed_multiplier = s;
                }
            }

            ui.separator();

            // Stats
            ui.label(format!(
                "Entities: {} | Food: {} | Tick: {}",
                sim.arena.count,
                sim.food.len(),
                sim.tick_count,
            ));

            ui.separator();

            // Panel toggles
            ui.toggle_value(&mut ui_state.show_inspector, "Inspector");
            ui.toggle_value(&mut ui_state.show_neural_viz, "Brain");
            ui.toggle_value(&mut ui_state.show_graphs, "Graphs");
            ui.toggle_value(&mut ui_state.show_minimap, "Minimap");
            ui.toggle_value(&mut ui_state.show_settings, "Settings");
        });
    });
}
