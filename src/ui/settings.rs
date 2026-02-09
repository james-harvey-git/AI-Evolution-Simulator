use egui;

use crate::simulation::SimState;
use crate::ui::UiState;
use crate::visual::{VisualQuality, VisualSettings};

/// Runtime settings content for the right-side dock.
pub fn draw_settings_content(
    ui: &mut egui::Ui,
    sim: &mut SimState,
    visual_settings: &mut VisualSettings,
    ui_state: &mut UiState,
) {
    ui.heading("Environment");
    ui.label(format!(
        "Time of day: {:.1}% ({})",
        sim.environment.time_of_day * 100.0,
        if sim.environment.is_day() {
            "Day"
        } else {
            "Night"
        }
    ));
    ui.label(format!("Season: {}", sim.environment.season.name()));
    ui.label(format!(
        "Season progress: {:.0}%",
        sim.environment.season_progress * 100.0
    ));

    ui.separator();
    ui.heading("Visual FX");
    egui::ComboBox::from_label("Quality")
        .selected_text(visual_settings.quality.label())
        .show_ui(ui, |ui| {
            for quality in VisualQuality::ALL {
                let selected = visual_settings.quality == quality;
                if ui.selectable_label(selected, quality.label()).clicked() {
                    visual_settings.set_quality_preset(quality);
                }
            }
        });
    ui.checkbox(&mut visual_settings.atmosphere_enabled, "Atmosphere overlay");
    ui.checkbox(&mut visual_settings.storm_fx_enabled, "Directional storm FX");
    ui.checkbox(
        &mut visual_settings.creature_detail_enabled,
        "Detailed creature rendering",
    );
    ui.checkbox(&mut visual_settings.trails_enabled, "Movement trails");
    ui.checkbox(
        &mut visual_settings.shelter_highlight_enabled,
        "Shelter range highlights",
    );

    ui.separator();
    ui.heading("Performance Policy");
    ui.checkbox(&mut ui_state.auto_quality_enabled, "Auto quality scaling");
    ui.checkbox(
        &mut ui_state.speed_render_decimation,
        "Speed-based render decimation",
    );
    egui::ComboBox::from_label("Quality min")
        .selected_text(ui_state.quality_min.label())
        .show_ui(ui, |ui| {
            for quality in VisualQuality::ALL {
                let selected = ui_state.quality_min == quality;
                if ui.selectable_label(selected, quality.label()).clicked() {
                    ui_state.quality_min = quality;
                }
            }
        });
    egui::ComboBox::from_label("Quality max")
        .selected_text(ui_state.quality_max.label())
        .show_ui(ui, |ui| {
            for quality in VisualQuality::ALL {
                let selected = ui_state.quality_max == quality;
                if ui.selectable_label(selected, quality.label()).clicked() {
                    ui_state.quality_max = quality;
                }
            }
        });
    ui.add(
        egui::Slider::new(&mut ui_state.max_sim_ms_per_frame, 2.0..=20.0)
            .logarithmic(false)
            .suffix(" ms")
            .text("Max sim time/frame"),
    );

    ui.separator();
    ui.heading("Spawn Tools");

    ui.horizontal(|ui| {
        if ui.button("Spawn 10 Food").clicked() {
            use ::rand::Rng;
            for _ in 0..10 {
                let pos = macroquad::prelude::vec2(
                    sim.rng.gen_range(0.0..sim.world.width),
                    sim.rng.gen_range(0.0..sim.world.height),
                );
                sim.food.push(crate::simulation::FoodItem {
                    pos,
                    energy: crate::config::FOOD_ENERGY,
                });
            }
        }
        if ui.button("Spawn 50 Food").clicked() {
            use ::rand::Rng;
            for _ in 0..50 {
                let pos = macroquad::prelude::vec2(
                    sim.rng.gen_range(0.0..sim.world.width),
                    sim.rng.gen_range(0.0..sim.world.height),
                );
                sim.food.push(crate::simulation::FoodItem {
                    pos,
                    energy: crate::config::FOOD_ENERGY,
                });
            }
        }
    });

    if ui.button("Spawn Entity").clicked() {
        use ::rand::Rng;
        let pos = macroquad::prelude::vec2(
            sim.rng.gen_range(50.0..sim.world.width - 50.0),
            sim.rng.gen_range(50.0..sim.world.height - 50.0),
        );
        let genome = crate::genome::Genome::random(&mut sim.rng);
        let entity =
            crate::entity::Entity::new_from_genome_rng(&genome, pos, sim.tick_count, &mut sim.rng);
        if let Some(id) = sim.arena.spawn(entity) {
            let slot = id.index as usize;
            sim.brains.init_from_genome(slot, &genome);
            if slot < sim.genomes.len() {
                sim.genomes[slot] = Some(genome);
            }
        }
    }

    if ui.button("Trigger Storm").clicked() {
        use ::rand::Rng;
        sim.environment.storm = Some(crate::environment::Storm {
            center: macroquad::prelude::vec2(
                sim.rng.gen_range(0.0..sim.world.width),
                sim.rng.gen_range(0.0..sim.world.height),
            ),
            radius: crate::config::STORM_RADIUS,
            velocity: macroquad::prelude::Vec2::from_angle(
                sim.rng.gen_range(0.0..std::f32::consts::TAU),
            ) * 30.0,
            timer: crate::config::STORM_DURATION,
        });
    }

    ui.separator();
    ui.heading("Info");
    ui.label(format!(
        "Spatial cells: {}x{}",
        sim.spatial_hash.cols, sim.spatial_hash.rows
    ));
    ui.label(format!(
        "Pheromone grid: {}x{}",
        sim.pheromone_grid.width, sim.pheromone_grid.height
    ));
    ui.label(format!("Walls: {}", sim.environment.walls.len()));
    ui.label(format!("Toxic zones: {}", sim.environment.toxic_zones.len()));
    ui.separator();
    ui.label("Tips:");
    ui.label("- Use toolbar tools: Select/Food/Hazard/Wall");
    ui.label("- Right click drops temporary hazard");
    ui.label("- Press E to spawn random entity at cursor");
}
