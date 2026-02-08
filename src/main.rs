use macroquad::prelude::*;

mod brain;
mod camera;
mod combat;
mod config;
mod energy;
mod entity;
mod environment;
mod genome;
mod particles;
mod physics;
mod post_processing;
mod renderer;
mod reproduction;
mod save_load;
mod sensory;
mod signals;
mod simulation;
mod spatial_hash;
mod stats;
mod ui;
mod world;

use camera::CameraController;
use simulation::SimState;
use stats::SimStats;
use ui::UiState;

fn window_conf() -> Conf {
    Conf {
        window_title: "GENESIS — Neural Evolution Simulator".to_string(),
        window_width: 1280,
        window_height: 800,
        window_resizable: true,
        high_dpi: true,
        ..Default::default()
    }
}

const AUTOSAVE_INTERVAL: f64 = 300.0; // 5 minutes

#[macroquad::main(window_conf)]
async fn main() {
    let mut sim = SimState::new(config::INITIAL_ENTITY_COUNT, 42);
    let mut camera = CameraController::new(sim.world.center());
    let mut accumulator = 0.0f64;
    let mut sim_stats = SimStats::new(1000);
    let mut ui_state = UiState::default();
    let mut bloom = post_processing::BloomPipeline::new();
    let mut autosave_timer = 0.0f64;

    loop {
        let frame_time = get_frame_time() as f64;
        accumulator += frame_time.min(0.1);

        // Autosave timer
        if !sim.paused {
            autosave_timer += frame_time;
            if autosave_timer >= AUTOSAVE_INTERVAL {
                autosave_timer = 0.0;
                match save_load::save_to_file(&sim, "genesis_autosave.bin") {
                    Ok(()) => eprintln!("[GENESIS] Autosaved to genesis_autosave.bin (tick {})", sim.tick_count),
                    Err(e) => eprintln!("[GENESIS] Autosave failed: {e}"),
                }
            }
        }

        let effective_dt = config::FIXED_DT as f64 / sim.speed_multiplier as f64;
        if !sim.paused {
            while accumulator >= effective_dt {
                sim.tick();

                // Record stats each tick
                let (avg_energy, avg_gen) = compute_averages(&sim);
                sim_stats.record(
                    sim.arena.count,
                    avg_energy,
                    sim.food.len(),
                    avg_gen,
                );

                accumulator -= effective_dt;
            }
        } else {
            accumulator = 0.0;
        }

        camera.update(&sim.arena, get_frame_time());

        // Entity selection via left click (only if egui doesn't want the input)
        let mut egui_wants_pointer = false;
        egui_macroquad::cfg(|ctx| {
            egui_wants_pointer = ctx.wants_pointer_input();
        });
        if !egui_wants_pointer && is_mouse_button_pressed(MouseButton::Left) {
            let mouse_screen = Vec2::from(mouse_position());
            let mouse_world = camera.screen_to_world(mouse_screen);
            let pick_radius = 30.0 / camera.smooth_zoom;
            if let Some(id) = camera.pick_entity(mouse_world, &sim.arena, pick_radius) {
                camera.following = Some(id);
            } else {
                camera.following = None;
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            camera.following = None;
        }

        if is_key_pressed(KeyCode::Space) {
            sim.paused = !sim.paused;
        }

        // Toggle sensor ray visualization
        if is_key_pressed(KeyCode::R) {
            sim.show_rays = !sim.show_rays;
        }

        // Delete selected entity
        if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
            if let Some(id) = camera.following {
                if let Some(entity) = sim.arena.get_mut(id) {
                    entity.alive = false;
                }
                camera.following = None;
            }
        }

        // Save/Load (Ctrl+S / Ctrl+L)
        if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
            if is_key_pressed(KeyCode::S) {
                match save_load::save_to_file(&sim, "genesis_save.bin") {
                    Ok(()) => eprintln!("[GENESIS] Saved to genesis_save.bin"),
                    Err(e) => eprintln!("[GENESIS] Save failed: {e}"),
                }
            }
            if is_key_pressed(KeyCode::L) {
                match save_load::load_from_file("genesis_save.bin") {
                    Ok(loaded) => {
                        sim = loaded;
                        camera = CameraController::new(sim.world.center());
                        eprintln!("[GENESIS] Loaded from genesis_save.bin (tick {})", sim.tick_count);
                    }
                    Err(e) => eprintln!("[GENESIS] Load failed: {e}"),
                }
            }
        }

        let alpha = if !sim.paused {
            (accumulator / effective_dt) as f32
        } else {
            1.0
        };

        // Render scene (with or without bloom)
        if let Some(ref mut b) = bloom {
            b.check_resize();
            renderer::draw_with_bloom(&sim, &camera, alpha, b);
        } else {
            renderer::draw(&sim, &camera, alpha);
        }

        // Draw egui UI on top
        ui::draw_ui(&mut sim, &mut camera, &mut ui_state, &sim_stats);

        next_frame().await;
    }
}

fn compute_averages(sim: &SimState) -> (f32, f32) {
    let mut total_energy = 0.0f32;
    let mut total_gen = 0.0f32;
    let mut count = 0u32;
    for (_idx, e) in sim.arena.iter_alive() {
        total_energy += e.energy;
        total_gen += e.generation_depth as f32;
        count += 1;
    }
    if count > 0 {
        (total_energy / count as f32, total_gen / count as f32)
    } else {
        (0.0, 0.0)
    }
}
