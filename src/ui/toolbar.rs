use egui;

use super::{ToolMode, UiState};
use crate::simulation::SimState;

/// Slim status strip + compact controls.
pub fn draw_toolbar(ctx: &egui::Context, sim: &mut SimState, ui_state: &mut UiState) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.add_space(3.0);
        ui.horizontal_wrapped(|ui| {
            title_badge(ui, "GENESIS");

            ui.separator();
            compact_group(ui, "Sim", |ui| {
                let pause_label = if sim.paused { "Play" } else { "Pause" };
                if ui.button(pause_label).clicked() {
                    sim.paused = !sim.paused;
                }
                if ui.button("Step").clicked() {
                    ui_state.step_requested = true;
                }
            });

            compact_group(ui, "Speed", |ui| {
                for speed in [1.0, 2.0, 5.0, 10.0] {
                    speed_button(ui, sim, speed);
                }
            });

            compact_group(ui, "Tool", |ui| {
                tool_button(ui, ui_state, ToolMode::Select, "Select");
                tool_button(ui, ui_state, ToolMode::SpawnFood, "Food");
                tool_button(ui, ui_state, ToolMode::SpawnHazard, "Hazard");
                tool_button(ui, ui_state, ToolMode::DrawWall, "Wall");
            });

            compact_group(ui, "Panels", |ui| {
                ui.toggle_value(&mut ui_state.show_dock, "Dock");
                ui.toggle_value(&mut ui_state.show_minimap, "Map");
                ui.toggle_value(&mut ui_state.show_inspector, "Inspector");
                ui.toggle_value(&mut ui_state.show_neural_viz, "Brain");
                ui.toggle_value(&mut ui_state.show_graphs, "Graphs");
                ui.toggle_value(&mut ui_state.show_settings, "Settings");
            });
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let high_speed_mode = sim.speed_multiplier >= 5.0;
            metric_chip(ui, "Population", format!("{}", sim.arena.count));
            metric_chip(ui, "Species", format!("{}", sim.cached_species_estimate));
            metric_chip(ui, "Avg Energy", format!("{:.0}", sim.cached_avg_energy));
            metric_chip(
                ui,
                "Season",
                format!(
                    "{} · {}",
                    sim.environment.season.name(),
                    if sim.environment.is_day() {
                        "Day"
                    } else {
                        "Night"
                    }
                ),
            );
            if sim.environment.storm.is_some() {
                status_chip(ui, "STORM", egui::Color32::from_rgb(98, 154, 191));
            }

            metric_chip(
                ui,
                "Quality",
                format!(
                    "{}{}",
                    ui_state.perf.quality.label(),
                    if ui_state.perf.auto_quality {
                        " (Auto)"
                    } else {
                        ""
                    }
                ),
            );
            metric_chip(ui, "Target Hz", format!("{:.0}", ui_state.perf.target_sim_hz));
            metric_chip(ui, "Render", format!("1/{}", ui_state.perf.render_every_n.max(1)));
            if !high_speed_mode {
                metric_chip(ui, "Frame", format!("{:.1}ms", ui_state.perf.frame_ms));
                metric_chip(ui, "Avg Age", format!("{:.0}s", sim.cached_avg_age));
                metric_chip(ui, "Avg Size", format!("{:.2}", sim.cached_avg_size));
                metric_chip(ui, "Gen", format!("{:.1}", sim.cached_avg_generation));
            }
            metric_chip(ui, "Ticks/frame", format!("{}", ui_state.perf.ticks_this_frame));
        });
        ui.add_space(3.0);
    });
}

fn speed_button(ui: &mut egui::Ui, sim: &mut SimState, speed: f32) {
    let label = format!("{speed}x");
    let selected = (sim.speed_multiplier - speed).abs() < 0.01;
    if ui.selectable_label(selected, label).clicked() {
        sim.speed_multiplier = speed;
    }
}

fn tool_button(ui: &mut egui::Ui, ui_state: &mut UiState, mode: ToolMode, label: &str) {
    let selected = ui_state.tool_mode == mode;
    if ui.selectable_label(selected, label).clicked() {
        ui_state.tool_mode = mode;
    }
}

fn title_badge(ui: &mut egui::Ui, label: &str) {
    let text = egui::RichText::new(label)
        .strong()
        .color(egui::Color32::from_rgb(190, 220, 255));
    ui.label(text);
}

fn compact_group(ui: &mut egui::Ui, heading: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(heading)
                    .small()
                    .color(egui::Color32::from_rgb(150, 170, 185)),
            );
            add_contents(ui);
        });
    });
}

fn metric_chip(ui: &mut egui::Ui, key: &str, value: String) {
    let text = egui::RichText::new(format!("{key}: {value}"))
        .small()
        .color(egui::Color32::from_rgb(205, 215, 225));
    ui.group(|ui| {
        ui.label(text);
    });
}

fn status_chip(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    ui.group(|ui| {
        ui.label(egui::RichText::new(label).small().strong().color(color));
    });
}
