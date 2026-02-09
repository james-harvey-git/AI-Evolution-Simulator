use macroquad::prelude::vec2;
use serde::Serialize;
use std::path::PathBuf;

use crate::camera::CameraController;
use crate::config;
use crate::entity::EntityId;
use crate::environment::Storm;
use crate::simulation::SimState;
use crate::ui::UiState;
use crate::visual::{VisualQuality, VisualSettings};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum QaScenario {
    Baseline,
    BoundaryProbe,
}

impl QaScenario {
    pub fn parse_cli(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "baseline" => Some(Self::Baseline),
            "boundary" | "boundary-probe" | "wrap-probe" => Some(Self::BoundaryProbe),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::BoundaryProbe => "boundary_probe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum QaPanel {
    Inspector,
    Brain,
    Graphs,
    Minimap,
    Settings,
}

impl QaPanel {
    fn label(self) -> &'static str {
        match self {
            QaPanel::Inspector => "Inspector",
            QaPanel::Brain => "Brain",
            QaPanel::Graphs => "Graphs",
            QaPanel::Minimap => "Minimap",
            QaPanel::Settings => "Settings",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QaAction {
    SetPanelVisibility { panel: QaPanel, visible: bool },
    SelectFirstEntity,
    ClearFollow,
    SpawnFoodCluster,
    SpawnToxicZone,
    AddWall,
    TriggerStorm,
    SetVisualQuality(VisualQuality),
    CaptureFrame { label: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledAction {
    pub tick: u64,
    pub action: QaAction,
}

#[derive(Debug, Clone, Serialize)]
pub struct QaActionLog {
    pub tick: u64,
    pub action: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QaCheck {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QaCapture {
    pub tick: u64,
    pub label: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QaReport {
    pub scenario: String,
    pub seed: u64,
    pub initial_entities: usize,
    pub final_tick: u64,
    pub visual_quality: String,
    pub action_count: usize,
    pub capture_count: usize,
    pub overall_status: String,
    pub checks: Vec<QaCheck>,
    pub actions: Vec<QaActionLog>,
    pub captures: Vec<QaCapture>,
}

pub struct QaDirector {
    scenario: QaScenario,
    schedule: Vec<ScheduledAction>,
    next_action_idx: usize,
    capture_request_queue: Vec<(u64, String)>,
    action_logs: Vec<QaActionLog>,
    checks: Vec<QaCheck>,
    captures: Vec<QaCapture>,
    max_tick: u64,
    output_dir: PathBuf,
    frames_dir: PathBuf,
    out_of_bounds_samples: u64,
    non_finite_samples: u64,
    rapid_turn_samples: u64,
    movement_samples: u64,
    prev_headings: Vec<Option<f32>>,
    behavior_checks_finalized: bool,
}

impl QaDirector {
    pub fn new(scenario: QaScenario, output_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("create QA output dir {} failed: {e}", output_dir.display()))?;
        let frames_dir = output_dir.join("qa_frames");
        std::fs::create_dir_all(&frames_dir)
            .map_err(|e| format!("create QA frames dir {} failed: {e}", frames_dir.display()))?;

        let schedule = build_schedule(scenario);
        let max_tick = schedule.iter().map(|s| s.tick).max().unwrap_or(0);
        Ok(Self {
            scenario,
            schedule,
            next_action_idx: 0,
            capture_request_queue: Vec::new(),
            action_logs: Vec::new(),
            checks: Vec::new(),
            captures: Vec::new(),
            max_tick,
            output_dir,
            frames_dir,
            out_of_bounds_samples: 0,
            non_finite_samples: 0,
            rapid_turn_samples: 0,
            movement_samples: 0,
            prev_headings: Vec::new(),
            behavior_checks_finalized: false,
        })
    }

    pub fn output_dir(&self) -> &PathBuf {
        &self.output_dir
    }

    pub fn run_actions_for_current_tick(
        &mut self,
        sim: &mut SimState,
        camera: &mut CameraController,
        ui_state: &mut UiState,
        visual: &mut VisualSettings,
    ) {
        while self.next_action_idx < self.schedule.len() {
            let tick = self.schedule[self.next_action_idx].tick;
            if tick > sim.tick_count {
                break;
            }
            if tick == sim.tick_count {
                let action = self.schedule[self.next_action_idx].action.clone();
                self.execute_action(tick, &action, sim, camera, ui_state, visual);
            }
            self.next_action_idx += 1;
        }
    }

    pub fn capture_pending_frames(&mut self) -> Result<(), String> {
        let requests: Vec<(u64, String)> = self.capture_request_queue.drain(..).collect();
        for (tick, label) in requests {
            let frame_idx = self.captures.len();
            let filename = format!("frame_{frame_idx:03}_{label}.png");
            let path = self.frames_dir.join(&filename);
            let image = macroquad::prelude::get_screen_data();
            image.export_png(&path.to_string_lossy());
            self.captures.push(QaCapture {
                tick,
                label,
                file: path.to_string_lossy().to_string(),
            });
        }
        Ok(())
    }

    pub fn is_complete(&self, tick_count: u64) -> bool {
        tick_count >= self.max_tick
            && self.next_action_idx >= self.schedule.len()
            && self.capture_request_queue.is_empty()
    }

    /// QA draw alpha override used to stress interpolation paths.
    pub fn render_alpha_override(&self, tick_count: u64) -> f32 {
        match self.scenario {
            QaScenario::Baseline => 1.0,
            QaScenario::BoundaryProbe => {
                if (20..=60).contains(&tick_count) {
                    1.35
                } else {
                    1.0
                }
            }
        }
    }

    pub fn observe_state(&mut self, sim: &SimState) {
        if self.prev_headings.len() != sim.arena.entities.len() {
            self.prev_headings.resize(sim.arena.entities.len(), None);
        }

        let base_turn_rate = config::ENTITY_TURN_RATE.max(0.001);
        for (idx, slot) in sim.arena.entities.iter().enumerate() {
            let entity = match slot {
                Some(e) if e.alive => e,
                _ => {
                    self.prev_headings[idx] = None;
                    continue;
                }
            };

            let in_bounds = entity.pos.x >= 0.0
                && entity.pos.x <= sim.world.width
                && entity.pos.y >= 0.0
                && entity.pos.y <= sim.world.height;
            if !in_bounds {
                self.out_of_bounds_samples += 1;
            }
            if !entity.pos.x.is_finite() || !entity.pos.y.is_finite() {
                self.non_finite_samples += 1;
            }

            if let Some(prev_heading) = self.prev_headings[idx] {
                let delta = wrap_angle(entity.heading - prev_heading).abs();
                let turn_rate = delta / config::FIXED_DT.max(1e-6);
                let speed = entity.velocity.length();
                let max_speed = config::ENTITY_MAX_SPEED * entity.speed_multiplier;
                if speed > max_speed * 0.4 {
                    self.movement_samples += 1;
                    if turn_rate > base_turn_rate * 0.92 {
                        self.rapid_turn_samples += 1;
                    }
                }
            }
            self.prev_headings[idx] = Some(entity.heading);
        }
    }

    pub fn finalize_behavior_checks(&mut self) {
        if self.behavior_checks_finalized {
            return;
        }
        self.behavior_checks_finalized = true;

        self.record_check(
            "entities_within_world_bounds".to_string(),
            self.out_of_bounds_samples == 0,
            format!("out_of_bounds_samples={}", self.out_of_bounds_samples),
        );
        self.record_check(
            "entity_positions_are_finite".to_string(),
            self.non_finite_samples == 0,
            format!("non_finite_samples={}", self.non_finite_samples),
        );
        let rapid_ratio = if self.movement_samples > 0 {
            self.rapid_turn_samples as f32 / self.movement_samples as f32
        } else {
            0.0
        };
        self.record_check(
            "rapid_turn_ratio_reasonable".to_string(),
            rapid_ratio <= 0.55,
            format!(
                "rapid_turn_samples={}, movement_samples={}, rapid_ratio={:.3}",
                self.rapid_turn_samples, self.movement_samples, rapid_ratio
            ),
        );
    }

    pub fn report(
        &self,
        seed: u64,
        initial_entities: usize,
        final_tick: u64,
        visual: VisualSettings,
    ) -> QaReport {
        let all_passed = self.checks.iter().all(|c| c.passed);
        QaReport {
            scenario: self.scenario.label().to_string(),
            seed,
            initial_entities,
            final_tick,
            visual_quality: visual.quality.label().to_string(),
            action_count: self.action_logs.len(),
            capture_count: self.captures.len(),
            overall_status: if all_passed { "PASS" } else { "FAIL" }.to_string(),
            checks: self.checks.clone(),
            actions: self.action_logs.clone(),
            captures: self.captures.clone(),
        }
    }

    fn execute_action(
        &mut self,
        tick: u64,
        action: &QaAction,
        sim: &mut SimState,
        camera: &mut CameraController,
        ui_state: &mut UiState,
        visual: &mut VisualSettings,
    ) {
        match action {
            QaAction::SetPanelVisibility { panel, visible } => {
                set_panel_visibility(ui_state, *panel, *visible);
                let actual = panel_visibility(ui_state, *panel);
                self.record_check(
                    format!(
                        "panel_{}_set_{}",
                        panel.label().to_ascii_lowercase(),
                        visible
                    ),
                    actual == *visible,
                    format!(
                        "expected {} visible={}, got {}",
                        panel.label(),
                        visible,
                        actual
                    ),
                );
                self.record_action(
                    tick,
                    format!("SetPanel({})", panel.label()),
                    format!("visible={visible}"),
                );
            }
            QaAction::SelectFirstEntity => {
                let maybe_follow = select_first_entity(sim);
                camera.following = maybe_follow;
                self.record_check(
                    "follow_selected_entity".to_string(),
                    camera.following.is_some(),
                    format!("following={:?}", camera.following),
                );
                self.record_action(
                    tick,
                    "SelectFirstEntity".to_string(),
                    format!("following={:?}", camera.following),
                );
            }
            QaAction::ClearFollow => {
                camera.following = None;
                self.record_check(
                    "follow_cleared".to_string(),
                    camera.following.is_none(),
                    "camera follow should be none".to_string(),
                );
                self.record_action(
                    tick,
                    "ClearFollow".to_string(),
                    "following=None".to_string(),
                );
            }
            QaAction::SpawnFoodCluster => {
                let before = sim.food.len();
                let pos = vec2(sim.world.width * 0.35, sim.world.height * 0.58);
                sim.spawn_food_cluster(pos, config::FOOD_CLUSTER_COUNT);
                let after = sim.food.len();
                self.record_check(
                    "food_spawned".to_string(),
                    after > before,
                    format!("before={before}, after={after}"),
                );
                self.record_action(tick, "SpawnFoodCluster".to_string(), format!("at={pos:?}"));
            }
            QaAction::SpawnToxicZone => {
                let before = sim.environment.toxic_zones.len();
                let pos = vec2(sim.world.width * 0.62, sim.world.height * 0.52);
                sim.spawn_toxic_zone(pos);
                let after = sim.environment.toxic_zones.len();
                self.record_check(
                    "toxic_zone_spawned".to_string(),
                    after > before,
                    format!("before={before}, after={after}"),
                );
                self.record_action(tick, "SpawnToxicZone".to_string(), format!("at={pos:?}"));
            }
            QaAction::AddWall => {
                let before = sim.environment.walls.len();
                let y = sim.world.height * 0.42;
                let start = vec2(sim.world.width * 0.2, y);
                let end = vec2(sim.world.width * 0.8, y + 35.0);
                sim.add_wall(start, end);
                let after = sim.environment.walls.len();
                self.record_check(
                    "wall_added".to_string(),
                    after > before,
                    format!("before={before}, after={after}"),
                );
                self.record_action(
                    tick,
                    "AddWall".to_string(),
                    format!("start={start:?}, end={end:?}"),
                );
            }
            QaAction::TriggerStorm => {
                sim.environment.storm = Some(Storm {
                    center: vec2(sim.world.width * 0.5, sim.world.height * 0.5),
                    radius: config::STORM_RADIUS,
                    velocity: vec2(24.0, -14.0),
                    timer: config::STORM_DURATION,
                });
                self.record_check(
                    "storm_triggered".to_string(),
                    sim.environment.storm.is_some(),
                    "storm should be active".to_string(),
                );
                self.record_action(tick, "TriggerStorm".to_string(), "storm=Some".to_string());
            }
            QaAction::SetVisualQuality(quality) => {
                visual.set_quality_preset(*quality);
                self.record_check(
                    "quality_changed".to_string(),
                    visual.quality == *quality,
                    format!(
                        "expected={}, got={}",
                        quality.label(),
                        visual.quality.label()
                    ),
                );
                self.record_action(
                    tick,
                    "SetVisualQuality".to_string(),
                    format!("quality={}", quality.label()),
                );
            }
            QaAction::CaptureFrame { label } => {
                self.capture_request_queue.push((tick, label.clone()));
                self.record_action(tick, "CaptureFrame".to_string(), format!("label={label}"));
            }
        }
    }

    fn record_action(&mut self, tick: u64, action: String, details: String) {
        self.action_logs.push(QaActionLog {
            tick,
            action,
            details,
        });
    }

    fn record_check(&mut self, name: String, passed: bool, details: String) {
        self.checks.push(QaCheck {
            name,
            passed,
            details,
        });
    }
}

fn select_first_entity(sim: &SimState) -> Option<EntityId> {
    sim.arena.iter_alive().next().map(|(idx, _)| EntityId {
        index: idx as u32,
        generation: sim.arena.generations[idx],
    })
}

fn set_panel_visibility(ui_state: &mut UiState, panel: QaPanel, visible: bool) {
    match panel {
        QaPanel::Inspector => ui_state.show_inspector = visible,
        QaPanel::Brain => ui_state.show_neural_viz = visible,
        QaPanel::Graphs => ui_state.show_graphs = visible,
        QaPanel::Minimap => ui_state.show_minimap = visible,
        QaPanel::Settings => ui_state.show_settings = visible,
    }
}

fn panel_visibility(ui_state: &UiState, panel: QaPanel) -> bool {
    match panel {
        QaPanel::Inspector => ui_state.show_inspector,
        QaPanel::Brain => ui_state.show_neural_viz,
        QaPanel::Graphs => ui_state.show_graphs,
        QaPanel::Minimap => ui_state.show_minimap,
        QaPanel::Settings => ui_state.show_settings,
    }
}

fn wrap_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

pub fn build_schedule(scenario: QaScenario) -> Vec<ScheduledAction> {
    match scenario {
        QaScenario::Baseline => baseline_schedule(),
        QaScenario::BoundaryProbe => boundary_probe_schedule(),
    }
}

fn baseline_schedule() -> Vec<ScheduledAction> {
    vec![
        ScheduledAction {
            tick: 0,
            action: QaAction::CaptureFrame {
                label: "baseline".to_string(),
            },
        },
        ScheduledAction {
            tick: 5,
            action: QaAction::SetPanelVisibility {
                panel: QaPanel::Inspector,
                visible: true,
            },
        },
        ScheduledAction {
            tick: 6,
            action: QaAction::SetPanelVisibility {
                panel: QaPanel::Brain,
                visible: true,
            },
        },
        ScheduledAction {
            tick: 7,
            action: QaAction::SetPanelVisibility {
                panel: QaPanel::Graphs,
                visible: true,
            },
        },
        ScheduledAction {
            tick: 8,
            action: QaAction::SetPanelVisibility {
                panel: QaPanel::Minimap,
                visible: true,
            },
        },
        ScheduledAction {
            tick: 9,
            action: QaAction::SetPanelVisibility {
                panel: QaPanel::Settings,
                visible: true,
            },
        },
        ScheduledAction {
            tick: 15,
            action: QaAction::SelectFirstEntity,
        },
        ScheduledAction {
            tick: 20,
            action: QaAction::SpawnFoodCluster,
        },
        ScheduledAction {
            tick: 25,
            action: QaAction::SpawnToxicZone,
        },
        ScheduledAction {
            tick: 30,
            action: QaAction::AddWall,
        },
        ScheduledAction {
            tick: 35,
            action: QaAction::CaptureFrame {
                label: "panel_rich".to_string(),
            },
        },
        ScheduledAction {
            tick: 45,
            action: QaAction::SetVisualQuality(VisualQuality::Ultra),
        },
        ScheduledAction {
            tick: 50,
            action: QaAction::TriggerStorm,
        },
        ScheduledAction {
            tick: 55,
            action: QaAction::CaptureFrame {
                label: "storm_active_fx".to_string(),
            },
        },
        ScheduledAction {
            tick: 75,
            action: QaAction::ClearFollow,
        },
        ScheduledAction {
            tick: 80,
            action: QaAction::SetVisualQuality(VisualQuality::High),
        },
    ]
}

fn boundary_probe_schedule() -> Vec<ScheduledAction> {
    vec![
        ScheduledAction {
            tick: 0,
            action: QaAction::CaptureFrame {
                label: "boundary_probe_start".to_string(),
            },
        },
        ScheduledAction {
            tick: 4,
            action: QaAction::SetPanelVisibility {
                panel: QaPanel::Minimap,
                visible: true,
            },
        },
        ScheduledAction {
            tick: 8,
            action: QaAction::SetVisualQuality(VisualQuality::High),
        },
        ScheduledAction {
            tick: 12,
            action: QaAction::SelectFirstEntity,
        },
        ScheduledAction {
            tick: 30,
            action: QaAction::CaptureFrame {
                label: "boundary_probe_alpha_stress".to_string(),
            },
        },
        ScheduledAction {
            tick: 45,
            action: QaAction::TriggerStorm,
        },
        ScheduledAction {
            tick: 55,
            action: QaAction::CaptureFrame {
                label: "boundary_probe_storm".to_string(),
            },
        },
        ScheduledAction {
            tick: 85,
            action: QaAction::CaptureFrame {
                label: "boundary_probe_end".to_string(),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_is_deterministic_for_same_scenario() {
        let a = build_schedule(QaScenario::Baseline);
        let b = build_schedule(QaScenario::Baseline);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn boundary_probe_schedule_is_deterministic() {
        let a = build_schedule(QaScenario::BoundaryProbe);
        let b = build_schedule(QaScenario::BoundaryProbe);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn director_runs_actions_for_expected_ticks() {
        let mut sim = SimState::new(10, 7);
        let mut cam = CameraController::new(sim.world.center());
        let mut ui = UiState::default();
        let mut visual = VisualSettings::default();
        let mut director = QaDirector::new(QaScenario::Baseline, std::env::temp_dir())
            .expect("qa director should initialize");

        sim.tick_count = 15;
        director.run_actions_for_current_tick(&mut sim, &mut cam, &mut ui, &mut visual);
        assert!(cam.following.is_some());
    }

    #[test]
    fn failed_check_sets_report_fail() {
        let mut director = QaDirector::new(QaScenario::Baseline, std::env::temp_dir())
            .expect("qa director should initialize");
        director.record_check("forced_failure".to_string(), false, "test".to_string());
        let report = director.report(42, 50, 100, VisualSettings::default());
        assert_eq!(report.overall_status, "FAIL");
    }

    #[test]
    fn render_alpha_override_stresses_boundary_probe_only() {
        let baseline = QaDirector::new(QaScenario::Baseline, std::env::temp_dir())
            .expect("qa director should initialize");
        assert!((baseline.render_alpha_override(30) - 1.0).abs() < 1e-6);

        let probe = QaDirector::new(QaScenario::BoundaryProbe, std::env::temp_dir())
            .expect("qa director should initialize");
        assert!(probe.render_alpha_override(30) > 1.0);
    }
}
