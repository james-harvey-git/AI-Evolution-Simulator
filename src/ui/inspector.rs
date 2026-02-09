use egui;

use crate::camera::CameraController;
use crate::config;
use crate::environment;
use crate::simulation::SimState;

/// Entity inspector content for the right-side dock.
pub fn draw_inspector_content(ui: &mut egui::Ui, sim: &SimState, camera: &CameraController) {
    ui.heading("Entity Inspector");
    ui.separator();

    if let Some(id) = camera.following {
        if let Some(entity) = sim.arena.get(id) {
            ui.label(format!("Slot: {} (gen {})", id.index, id.generation));
            ui.separator();

            ui.collapsing("Position & Movement", |ui| {
                ui.label(format!("Pos: ({:.0}, {:.0})", entity.pos.x, entity.pos.y));
                ui.label(format!("Heading: {:.1}°", entity.heading.to_degrees()));
                ui.label(format!("Speed: {:.1}", entity.velocity.length()));
                ui.label(format!("Radius: {:.1}", entity.radius));
            });

            ui.separator();
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
                ui.label(format!("Carried Energy: {:.1}", entity.carried_energy));

                if sim.environment.storm.is_some() {
                    let shelter_mult = environment::combined_storm_shelter_multiplier(
                        entity.pos,
                        &sim.environment.terrain,
                        &sim.environment.walls,
                        &sim.world,
                    );
                    ui.label(format!("Storm Shelter Mult: {:.2}x", shelter_mult));
                }
            });

            ui.separator();
            ui.collapsing("Genome Traits", |ui| {
                let slot = id.index as usize;
                if let Some(Some(genome)) = sim.genomes.get(slot) {
                    ui.label(format!("Neurons: {}", genome.total_neurons()));
                    ui.label(format!("Interneurons: {}", genome.inter_neurons()));
                    ui.label(format!("Body size: {:.2}", genome.body_size()));
                    ui.label(format!("Max speed: {:.2}", genome.max_speed()));
                    ui.label(format!("Metabolic rate: {:.2}", genome.metabolic_rate()));
                    ui.label(format!("Sensor range: {:.2}", genome.sensor_range()));
                    ui.label(format!("Mutation rate: {:.3}", genome.mutation_rate()));
                    ui.label(format!("Mutation sigma: {:.3}", genome.mutation_sigma()));

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
            ui.collapsing("Brain Outputs", |ui| {
                let slot = id.index as usize;
                if slot < sim.brains.active.len() && sim.brains.active[slot] {
                    if let Some(n) = sim.brains.neuron_count(slot) {
                        ui.label(format!("Brain neurons: {}", n));
                    }
                    if let Some(inter) = sim.brains.interneuron_count(slot) {
                        ui.label(format!("Brain inter: {}", inter));
                    }
                    let outputs = sim.brains.motor_outputs(slot);
                    ui.label(format!("Forward: {:.2}", outputs.forward));
                    ui.label(format!("Turn: {:.2}", outputs.turn));
                    ui.label(format!("Eat: {:.2}", outputs.eat));
                    ui.label(format!("Attack: {:.2}", outputs.attack));
                    ui.label(format!("Share: {:.2}", outputs.share));
                    ui.label(format!("Pickup: {:.2}", outputs.pickup));
                    ui.label(format!("Reproduce: {:.2}", outputs.reproduce));
                    ui.label(format!(
                        "Signal RGB: {:.2}, {:.2}, {:.2}",
                        outputs.signal_rgb[0], outputs.signal_rgb[1], outputs.signal_rgb[2]
                    ));
                }
            });
        } else {
            ui.label("Selected entity is dead.");
            ui.label("Press Escape to clear selection.");
        }
    } else {
        ui.label("Click an entity to inspect it.");
        ui.label("Press Escape to deselect.");

        ui.separator();
        ui.heading("Population Summary");
        ui.label(format!("Avg energy: {:.1}", sim.cached_avg_energy));
        ui.label(format!("Avg generation: {:.1}", sim.cached_avg_generation));
        ui.label(format!("Species est.: {}", sim.cached_species_estimate));
        ui.label(format!("Meat items: {}", sim.meat.len()));
        ui.label(format!(
            "Season: {} | {}",
            sim.environment.season.name(),
            if sim.environment.is_day() {
                "Day"
            } else {
                "Night"
            }
        ));
        if sim.environment.storm.is_some() {
            ui.colored_label(egui::Color32::from_rgb(200, 180, 100), "STORM ACTIVE");
        }
    }
}
