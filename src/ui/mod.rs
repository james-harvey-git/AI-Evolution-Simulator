pub mod graphs;
pub mod inspector;
pub mod minimap;
pub mod neural_viz;
pub mod settings;
pub mod toolbar;

use macroquad::prelude::Vec2;

use crate::camera::CameraController;
use crate::simulation::SimState;
use crate::stats::SimStats;
use crate::visual::{VisualQuality, VisualSettings};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    Select,
    SpawnFood,
    SpawnHazard,
    DrawWall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockTab {
    Inspector,
    Brain,
    Graphs,
    Settings,
}

impl DockTab {
    pub const ALL: [Self; 4] = [Self::Inspector, Self::Brain, Self::Graphs, Self::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Self::Inspector => "Inspector",
            Self::Brain => "Brain",
            Self::Graphs => "Graphs",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy)]
pub struct PerformanceHudState {
    pub quality: VisualQuality,
    pub target_sim_hz: f32,
    pub render_every_n: u32,
    pub auto_quality: bool,
    pub frame_ms: f32,
    pub ticks_this_frame: u32,
}

impl Default for PerformanceHudState {
    fn default() -> Self {
        Self {
            quality: VisualQuality::High,
            target_sim_hz: 60.0,
            render_every_n: 1,
            auto_quality: true,
            frame_ms: 16.7,
            ticks_this_frame: 0,
        }
    }
}

/// Tracks which UI panels are open.
pub struct UiState {
    pub show_inspector: bool,
    pub show_graphs: bool,
    pub show_minimap: bool,
    pub show_settings: bool,
    pub show_neural_viz: bool,
    pub show_dock: bool,
    pub active_dock_tab: DockTab,
    pub step_requested: bool,
    pub tool_mode: ToolMode,
    pub wall_drag_start: Option<Vec2>,
    pub perf: PerformanceHudState,
    pub auto_quality_enabled: bool,
    pub speed_render_decimation: bool,
    pub quality_min: VisualQuality,
    pub quality_max: VisualQuality,
    pub max_sim_ms_per_frame: f64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_inspector: true,
            show_graphs: false,
            show_minimap: true,
            show_settings: false,
            show_neural_viz: false,
            show_dock: true,
            active_dock_tab: DockTab::Inspector,
            step_requested: false,
            tool_mode: ToolMode::Select,
            wall_drag_start: None,
            perf: PerformanceHudState::default(),
            auto_quality_enabled: true,
            speed_render_decimation: true,
            quality_min: VisualQuality::Low,
            quality_max: VisualQuality::Ultra,
            max_sim_ms_per_frame: 7.5,
        }
    }
}

impl UiState {
    pub fn set_runtime_quality_policy(
        &mut self,
        auto_quality_enabled: bool,
        speed_render_decimation: bool,
        quality_min: VisualQuality,
        quality_max: VisualQuality,
        max_sim_ms_per_frame: f64,
    ) {
        self.auto_quality_enabled = auto_quality_enabled;
        self.speed_render_decimation = speed_render_decimation;
        self.quality_min = quality_min;
        self.quality_max = quality_max;
        self.max_sim_ms_per_frame = max_sim_ms_per_frame.max(1.0);
    }

    pub fn set_perf_snapshot(
        &mut self,
        quality: VisualQuality,
        target_sim_hz: f32,
        render_every_n: u32,
        auto_quality: bool,
        frame_ms: f32,
        ticks_this_frame: u32,
    ) {
        self.perf = PerformanceHudState {
            quality,
            target_sim_hz,
            render_every_n,
            auto_quality,
            frame_ms,
            ticks_this_frame,
        };
    }
}

/// Draw all egui UI panels.
pub fn draw_ui(
    sim: &mut SimState,
    camera: &mut CameraController,
    ui_state: &mut UiState,
    stats: &SimStats,
    visual_settings: &mut VisualSettings,
) {
    egui_macroquad::ui(|ctx| {
        apply_cinematic_theme(ctx);
        toolbar::draw_toolbar(ctx, sim, ui_state);

        draw_dock(ctx, sim, camera, ui_state, stats, visual_settings);
        if ui_state.show_minimap {
            minimap::draw_minimap(ctx, sim, camera);
        }
    });

    egui_macroquad::draw();
}

fn apply_cinematic_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(7.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.menu_margin = egui::Margin::same(6);
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = egui::Color32::from_rgb(14, 19, 29);
    style.visuals.window_fill = egui::Color32::from_rgb(16, 23, 34);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(9, 12, 18);
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(21, 27, 40);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(28, 36, 52);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(37, 48, 69);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 67, 94);
    style.visuals.widgets.open.bg_fill = egui::Color32::from_rgb(35, 47, 68);
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(205, 218, 235);
    style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(228, 236, 246);
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(239, 244, 250);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(58, 98, 144);
    style.visuals.selection.stroke.color = egui::Color32::from_rgb(197, 224, 255);
    style.visuals.window_stroke.color = egui::Color32::from_rgb(51, 69, 96);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
    style.visuals.window_corner_radius = egui::CornerRadius::same(6);
    ctx.set_style(style);
}

fn draw_dock(
    ctx: &egui::Context,
    sim: &mut SimState,
    camera: &mut CameraController,
    ui_state: &mut UiState,
    stats: &SimStats,
    visual_settings: &mut VisualSettings,
) {
    if !ui_state.show_dock {
        return;
    }

    egui::SidePanel::right("right_dock")
        .default_width(340.0)
        .min_width(260.0)
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for tab in DockTab::ALL {
                    let selected = ui_state.active_dock_tab == tab;
                    if ui.selectable_label(selected, tab.label()).clicked() {
                        ui_state.active_dock_tab = tab;
                        match tab {
                            DockTab::Inspector => ui_state.show_inspector = true,
                            DockTab::Brain => ui_state.show_neural_viz = true,
                            DockTab::Graphs => ui_state.show_graphs = true,
                            DockTab::Settings => ui_state.show_settings = true,
                        }
                    }
                }
                ui.separator();
                ui.checkbox(&mut ui_state.show_dock, "Dock");
            });
            ui.separator();

            match ui_state.active_dock_tab {
                DockTab::Inspector => {
                    if ui_state.show_inspector {
                        inspector::draw_inspector_content(ui, sim, camera);
                    } else {
                        ui.label("Inspector hidden from toolbar controls.");
                    }
                }
                DockTab::Brain => {
                    if ui_state.show_neural_viz {
                        if let Some(id) = camera.following {
                            neural_viz::draw_neural_viz_content(ui, &sim.brains, id.index as usize);
                        } else {
                            ui.label("Select an entity to view its brain.");
                        }
                    } else {
                        ui.label("Brain panel hidden from toolbar controls.");
                    }
                }
                DockTab::Graphs => {
                    if ui_state.show_graphs {
                        graphs::draw_graphs_content(ui, stats);
                    } else {
                        ui.label("Graphs panel hidden from toolbar controls.");
                    }
                }
                DockTab::Settings => {
                    if ui_state.show_settings {
                        settings::draw_settings_content(ui, sim, visual_settings, ui_state);
                    } else {
                        ui.label("Settings panel hidden from toolbar controls.");
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ui_starts_with_inspector_tab() {
        let ui = UiState::default();
        assert_eq!(ui.active_dock_tab, DockTab::Inspector);
        assert!(ui.show_dock);
    }

    #[test]
    fn perf_snapshot_updates_state() {
        let mut ui = UiState::default();
        ui.set_perf_snapshot(VisualQuality::Low, 300.0, 3, true, 22.0, 5);
        assert_eq!(ui.perf.quality, VisualQuality::Low);
        assert_eq!(ui.perf.render_every_n, 3);
        assert_eq!(ui.perf.ticks_this_frame, 5);
    }

    #[test]
    fn runtime_quality_policy_can_be_updated() {
        let mut ui = UiState::default();
        ui.set_runtime_quality_policy(
            false,
            false,
            VisualQuality::Medium,
            VisualQuality::High,
            12.0,
        );
        assert!(!ui.auto_quality_enabled);
        assert!(!ui.speed_render_decimation);
        assert_eq!(ui.quality_min, VisualQuality::Medium);
        assert_eq!(ui.quality_max, VisualQuality::High);
        assert!((ui.max_sim_ms_per_frame - 12.0).abs() < 1e-6);
    }
}
