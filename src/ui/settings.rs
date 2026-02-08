use egui;

use crate::simulation::SimState;

/// Runtime settings panel for tuning simulation parameters.
pub fn draw_settings(ctx: &egui::Context, sim: &mut SimState) {
    egui::Window::new("Settings")
        .default_pos(egui::pos2(300.0, 60.0))
        .default_size(egui::vec2(280.0, 360.0))
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Environment");

            // Day/night speed (modify day_progress rate indirectly by showing current state)
            ui.label(format!(
                "Time of day: {:.1}% ({})",
                sim.environment.time_of_day * 100.0,
                if sim.environment.is_day() { "Day" } else { "Night" }
            ));
            ui.label(format!("Season: {}", sim.environment.season.name()));
            ui.label(format!(
                "Season progress: {:.0}%",
                sim.environment.season_progress * 100.0
            ));

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
                let entity = crate::entity::Entity::new_from_genome_rng(
                    &genome,
                    pos,
                    sim.tick_count,
                    &mut sim.rng,
                );
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
            ui.label(format!("Spatial cells: {}x{}", sim.spatial_hash.cols, sim.spatial_hash.rows));
            ui.label(format!("Pheromone grid: {}x{}", sim.pheromone_grid.width, sim.pheromone_grid.height));
        });
}
