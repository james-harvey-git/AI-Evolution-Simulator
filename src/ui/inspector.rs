use egui;

use crate::camera::CameraController;
use crate::config;
use crate::simulation::SimState;

/// Entity inspector panel: shows stats for the selected (followed) entity.
pub fn draw_inspector(
    ctx: &egui::Context,
    sim: &SimState,
    camera: &CameraController,
) {
    egui::SidePanel::left("inspector")
        .default_width(220.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Entity Inspector");
            ui.separator();

            if let Some(id) = camera.following {
                if let Some(entity) = sim.arena.get(id) {
                    ui.label(format!("Slot: {} (gen {})", id.index, id.generation));
                    ui.separator();

                    // Position & movement
                    ui.collapsing("Position & Movement", |ui| {
                        ui.label(format!("Pos: ({:.0}, {:.0})", entity.pos.x, entity.pos.y));
                        ui.label(format!("Heading: {:.1}°", entity.heading.to_degrees()));
                        let speed = entity.velocity.length();
                        ui.label(format!("Speed: {:.1}", speed));
                        ui.label(format!("Radius: {:.1}", entity.radius));
                    });

                    ui.separator();

                    // Vitals
                    ui.collapsing("Vitals", |ui| {
                        let energy_frac = entity.energy / config::MAX_ENTITY_ENERGY;
                        ui.horizontal(|ui| {
                            ui.label("Energy:");
                            let bar = egui::ProgressBar::new(energy_frac.clamp(0.0, 1.0))
                                .text(format!("{:.0}/{:.0}", entity.energy, config::MAX_ENTITY_ENERGY));
                            ui.add(bar);
                        });

                        let health_frac = entity.health / entity.max_health;
                        ui.horizontal(|ui| {
                            ui.label("Health:");
                            let bar = egui::ProgressBar::new(health_frac.clamp(0.0, 1.0))
                                .text(format!("{:.0}/{:.0}", entity.health, entity.max_health));
                            ui.add(bar);
                        });

                        ui.label(format!("Age: {:.0}s", entity.age));
                    });

                    ui.separator();

                    // Genome traits
                    ui.collapsing("Genome Traits", |ui| {
                        let slot = id.index as usize;
                        if let Some(Some(genome)) = sim.genomes.get(slot) {
                            ui.label(format!("Body size: {:.2}", genome.body_size()));
                            ui.label(format!("Max speed: {:.2}", genome.max_speed()));
                            ui.label(format!("Metabolic rate: {:.2}", genome.metabolic_rate()));
                            ui.label(format!("Sensor range: {:.2}", genome.sensor_range()));
                            ui.label(format!("Mutation rate: {:.3}", genome.mutation_rate()));

                            let c = genome.body_color();
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                let rect = ui.allocate_space(egui::vec2(20.0, 14.0));
                                ui.painter().rect_filled(
                                    rect.1,
                                    0.0,
                                    egui::Color32::from_rgb(
                                        (c.r * 255.0) as u8,
                                        (c.g * 255.0) as u8,
                                        (c.b * 255.0) as u8,
                                    ),
                                );
                            });
                        }
                    });

                    ui.separator();

                    // Lineage
                    ui.collapsing("Lineage", |ui| {
                        ui.label(format!("Generation: {}", entity.generation_depth));
                        ui.label(format!("Offspring: {}", entity.offspring_count));
                        if let Some(pid) = entity.parent_id {
                            ui.label(format!("Parent: slot {}", pid.index));
                        } else {
                            ui.label("Parent: (original)");
                        }
                    });

                    ui.separator();

                    // Brain outputs
                    ui.collapsing("Brain Outputs", |ui| {
                        let slot = id.index as usize;
                        if slot < sim.brains.active.len() && sim.brains.active[slot] {
                            let (fwd, turn, attack, signal) = sim.brains.motor_outputs(slot);
                            ui.label(format!("Forward: {:.2}", fwd));
                            ui.label(format!("Turn: {:.2}", turn));
                            ui.label(format!("Attack: {:.2}", attack));
                            ui.label(format!("Signal: {:.2}", signal));
                        }
                    });
                } else {
                    ui.label("Selected entity is dead.");
                    if ui.button("Clear selection").clicked() {
                        // Can't mutate camera here, but user can press Escape
                    }
                }
            } else {
                ui.label("Click an entity to inspect it.");
                ui.label("Press Escape to deselect.");

                ui.separator();
                ui.heading("Population Summary");

                let mut total_energy = 0.0f32;
                let mut total_gen = 0u64;
                let mut count = 0u32;
                for (_idx, e) in sim.arena.iter_alive() {
                    total_energy += e.energy;
                    total_gen += e.generation_depth as u64;
                    count += 1;
                }

                if count > 0 {
                    ui.label(format!("Avg energy: {:.1}", total_energy / count as f32));
                    ui.label(format!("Avg generation: {:.1}", total_gen as f32 / count as f32));
                }

                ui.label(format!("Meat items: {}", sim.meat.len()));
                ui.label(format!(
                    "Season: {} | {}",
                    sim.environment.season.name(),
                    if sim.environment.is_day() { "Day" } else { "Night" }
                ));
                if sim.environment.storm.is_some() {
                    ui.colored_label(egui::Color32::from_rgb(200, 180, 100), "STORM ACTIVE");
                }
            }
        });
}
