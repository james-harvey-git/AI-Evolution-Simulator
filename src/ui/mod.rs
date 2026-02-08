pub mod toolbar;
pub mod inspector;
pub mod neural_viz;
pub mod graphs;
pub mod minimap;
pub mod settings;

use crate::camera::CameraController;
use crate::simulation::SimState;
use crate::stats::SimStats;

/// Tracks which UI panels are open.
pub struct UiState {
    pub show_inspector: bool,
    pub show_graphs: bool,
    pub show_minimap: bool,
    pub show_settings: bool,
    pub show_neural_viz: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_inspector: true,
            show_graphs: false,
            show_minimap: true,
            show_settings: false,
            show_neural_viz: false,
        }
    }
}

/// Draw all egui UI panels.
pub fn draw_ui(
    sim: &mut SimState,
    camera: &mut CameraController,
    ui_state: &mut UiState,
    stats: &SimStats,
) {
    egui_macroquad::ui(|ctx| {
        toolbar::draw_toolbar(ctx, sim, ui_state);

        if ui_state.show_inspector {
            inspector::draw_inspector(ctx, sim, camera);
        }

        if ui_state.show_neural_viz {
            if let Some(id) = camera.following {
                neural_viz::draw_neural_viz(ctx, &sim.brains, id.index as usize);
            }
        }

        if ui_state.show_graphs {
            graphs::draw_graphs(ctx, stats);
        }

        if ui_state.show_minimap {
            minimap::draw_minimap(ctx, sim, camera);
        }

        if ui_state.show_settings {
            settings::draw_settings(ctx, sim);
        }
    });

    egui_macroquad::draw();
}
