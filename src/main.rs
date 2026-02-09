use macroquad::prelude::*;
use std::path::PathBuf;

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
mod qa;
mod renderer;
mod reporting;
mod reproduction;
mod save_load;
mod sensory;
mod signals;
mod simulation;
mod spatial_hash;
mod stats;
mod ui;
mod visual;
mod world;

use camera::CameraController;
use qa::{QaDirector, QaScenario};
use reporting::{MetricAggregator, MetricSummary};
use serde::Serialize;
use simulation::SimState;
use stats::SimStats;
use ui::{ToolMode, UiState};
use visual::{
    AdaptiveQualityConfig, AdaptiveQualityController, VisualQuality, VisualQualityBounds,
    VisualSettings,
};

#[derive(Debug)]
struct RunConfig {
    seed: u64,
    initial_entities: usize,
    snapshot: Option<SnapshotSettings>,
    qa: Option<QaSettings>,
    benchmark: Option<BenchmarkSettings>,
    disable_bloom: bool,
    visual: VisualSettings,
    auto_quality: bool,
    quality_bounds: VisualQualityBounds,
    speed_render_decimation: bool,
    max_sim_ms_per_frame: f64,
    render_every_n_frames: Option<u32>,
}

#[derive(Debug, Clone)]
struct QaSettings {
    scenario: QaScenario,
    output_dir: PathBuf,
    show_ui: bool,
}

#[derive(Debug, Clone)]
struct BenchmarkSettings {
    output_dir: PathBuf,
    run_seconds: u32,
    warmup_seconds: u32,
    show_ui: bool,
}

#[derive(Debug)]
struct SnapshotSettings {
    output_dir: PathBuf,
    capture_ticks: Vec<u64>,
    next_capture_idx: usize,
    max_tick: u64,
    hide_ui: bool,
    samples: Vec<SnapshotSample>,
}

#[derive(Debug)]
struct SnapshotSample {
    tick: u64,
    population: usize,
    food: usize,
    meat: usize,
    avg_energy: f32,
    avg_age: f32,
    avg_generation: f32,
    avg_size: f32,
    species_estimate: usize,
    storm_active: bool,
    wall_count: usize,
    toxic_zone_count: usize,
    season: String,
    is_day: bool,
}

#[derive(Debug, Clone)]
struct BenchmarkFrameSample {
    frame_idx: u64,
    elapsed_sec: f64,
    sim_tick: u64,
    ticks_this_frame: u32,
    frame_ms: f64,
    fps: f64,
    population: usize,
}

#[derive(Debug, Serialize)]
struct BenchmarkRunMeta {
    seed: u64,
    initial_entities: usize,
    run_seconds: u32,
    warmup_seconds: u32,
    quality: String,
    show_ui: bool,
    atmosphere: bool,
    storm_fx: bool,
    creature_detail: bool,
    trails: bool,
    shelter_highlight: bool,
    auto_quality: bool,
    quality_min: String,
    quality_max: String,
    speed_render_decimation: bool,
    max_sim_ms_per_frame: f64,
}

#[derive(Debug, Serialize)]
struct BenchmarkPassCriteria {
    target_entities_ok: bool,
    avg_fps_ok: bool,
    p95_frame_ms_ok: bool,
    overall: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    meta: BenchmarkRunMeta,
    measured_frames: usize,
    total_sim_ticks: u64,
    avg_ticks_per_second: f64,
    frame_ms: MetricSummary,
    fps: MetricSummary,
    ticks_per_frame: MetricSummary,
    frames_under_16_7ms_pct: f64,
    criteria: BenchmarkPassCriteria,
}

#[derive(Debug, Clone, Copy)]
struct SpeedPolicy;

impl SpeedPolicy {
    fn new() -> Self {
        Self
    }

    fn target_sim_hz(self, speed_multiplier: f32) -> f64 {
        let s = speed_multiplier.max(1.0);
        if s >= 10.0 {
            600.0
        } else if s >= 5.0 {
            300.0
        } else if s >= 2.0 {
            120.0
        } else {
            60.0
        }
    }

    fn render_every_n_frames(self, speed_multiplier: f32, enabled: bool) -> u32 {
        if !enabled {
            return 1;
        }
        let s = speed_multiplier.max(1.0);
        if s >= 10.0 {
            3
        } else if s >= 5.0 {
            2
        } else {
            1
        }
    }

    fn quality_cap_for_speed(self, speed_multiplier: f32) -> VisualQuality {
        let s = speed_multiplier.max(1.0);
        if s >= 10.0 {
            VisualQuality::Low
        } else if s >= 5.0 {
            VisualQuality::Medium
        } else if s >= 2.0 {
            VisualQuality::High
        } else {
            VisualQuality::Ultra
        }
    }
}

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
const MAX_SIM_ACCUMULATOR_STEPS: f64 = 3.0;

impl SnapshotSettings {
    fn new(output_dir: PathBuf, mut capture_ticks: Vec<u64>, hide_ui: bool) -> Self {
        if capture_ticks.is_empty() {
            capture_ticks.push(0);
        }
        capture_ticks.sort_unstable();
        capture_ticks.dedup();

        if let Err(e) = std::fs::create_dir_all(&output_dir) {
            eprintln!(
                "[GENESIS] Failed to create snapshot output directory {}: {e}",
                output_dir.display()
            );
        }

        Self {
            max_tick: *capture_ticks.last().unwrap_or(&0),
            output_dir,
            capture_ticks,
            next_capture_idx: 0,
            hide_ui,
            samples: Vec::new(),
        }
    }

    fn should_capture(&self, tick: u64) -> bool {
        self.next_capture_idx < self.capture_ticks.len()
            && self.capture_ticks[self.next_capture_idx] == tick
    }

    fn capture_current_frame(&mut self, sim: &SimState) {
        if !self.should_capture(sim.tick_count) {
            return;
        }

        let frame_idx = self.next_capture_idx;
        let tick = sim.tick_count;
        let filename = format!("frame_{frame_idx:03}_tick_{tick:06}.png");
        let path = self.output_dir.join(filename);

        let image = get_screen_data();
        image.export_png(&path.to_string_lossy());

        self.samples.push(snapshot_sample(sim));
        self.next_capture_idx += 1;

        eprintln!(
            "[GENESIS] Snapshot captured: {} (tick {})",
            path.display(),
            tick
        );
    }

    fn is_complete(&self, sim_tick: u64) -> bool {
        sim_tick >= self.max_tick && self.next_capture_idx >= self.capture_ticks.len()
    }

    fn write_report(
        &self,
        sim: &SimState,
        seed: u64,
        initial_entities: usize,
        visual: VisualSettings,
    ) -> Result<(), String> {
        let report_path = self.output_dir.join("snapshot_report.csv");
        let mut csv = String::from(
            "tick,population,food,meat,avg_energy,avg_age,avg_generation,avg_size,species_estimate,storm_active,wall_count,toxic_zone_count,season,is_day\n",
        );
        for s in &self.samples {
            csv.push_str(&format!(
                "{},{},{},{},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{}\n",
                s.tick,
                s.population,
                s.food,
                s.meat,
                s.avg_energy,
                s.avg_age,
                s.avg_generation,
                s.avg_size,
                s.species_estimate,
                s.storm_active,
                s.wall_count,
                s.toxic_zone_count,
                s.season,
                s.is_day
            ));
        }
        std::fs::write(&report_path, csv)
            .map_err(|e| format!("write snapshot_report.csv failed: {e}"))?;

        let (avg_energy, avg_gen, avg_age, avg_size) = compute_population_summary(sim);
        let summary_path = self.output_dir.join("summary.txt");
        let summary = format!(
            "seed={seed}\ninitial_entities={initial_entities}\nfinal_tick={}\nrequested_capture_ticks={:?}\ncaptures={}\nfinal_population={}\nfinal_food={}\nfinal_meat={}\nfinal_avg_energy={:.3}\nfinal_avg_generation={:.3}\nfinal_avg_age={:.3}\nfinal_avg_size={:.3}\nfinal_species_estimate={}\nstorm_active={}\nwalls={}\ntoxic_zones={}\nseason={}\nis_day={}\nvisual_quality={}\natmosphere={}\nstorm_fx={}\ncreature_detail={}\ntrails={}\nshelter_highlight={}\n",
            sim.tick_count,
            self.capture_ticks,
            self.samples.len(),
            sim.arena.count,
            sim.food.len(),
            sim.meat.len(),
            avg_energy,
            avg_gen,
            avg_age,
            avg_size,
            sim.cached_species_estimate,
            sim.environment.storm.is_some(),
            sim.environment.walls.len(),
            sim.environment.toxic_zones.len(),
            sim.environment.season.name(),
            sim.environment.is_day(),
            visual.quality.label(),
            visual.atmosphere_enabled,
            visual.storm_fx_enabled,
            visual.creature_detail_enabled,
            visual.trails_enabled,
            visual.shelter_highlight_enabled,
        );
        std::fs::write(&summary_path, summary)
            .map_err(|e| format!("write summary.txt failed: {e}"))?;
        Ok(())
    }
}

fn parse_ticks_csv(input: &str) -> Vec<u64> {
    let mut ticks = Vec::new();
    for token in input.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Ok(value) = token.parse::<u64>() {
            ticks.push(value);
        }
    }
    ticks
}

fn clamp_sim_accumulator(accumulator: f64, effective_dt: f64) -> f64 {
    if effective_dt <= f64::EPSILON {
        return 0.0;
    }
    accumulator.min(effective_dt * MAX_SIM_ACCUMULATOR_STEPS)
}

fn parse_run_config_from_args(args: &[String]) -> RunConfig {
    let mut seed = 42u64;
    let mut initial_entities = config::INITIAL_ENTITY_COUNT;
    let mut snapshot_enabled = false;
    let mut snapshot_ticks = vec![0, 300, 1000, 3000, 5000];
    let mut snapshot_out = PathBuf::from("snapshot_runs/latest");
    let mut snapshot_hide_ui = true;
    let mut disable_bloom = false;
    let mut visual = VisualSettings::default();
    let mut qa_enabled = false;
    let mut qa_out = PathBuf::from("qa_runs/latest");
    let mut qa_scenario = QaScenario::Baseline;
    let mut qa_show_ui = true;
    let mut benchmark_enabled = false;
    let mut benchmark_out = PathBuf::from("benchmark_runs/latest");
    let mut benchmark_run_seconds = 60u32;
    let mut benchmark_warmup_seconds = 5u32;
    let mut benchmark_show_ui = false;
    let mut auto_quality = true;
    let mut quality_min = VisualQuality::Low;
    let mut quality_max = VisualQuality::Ultra;
    let mut speed_render_decimation = true;
    let mut max_sim_ms_per_frame = 7.5f64;
    let mut render_every_n_frames: Option<u32> = None;

    let mut i = 1usize;
    while i < args.len() {
        let arg = args[i].as_str();
        let mut consume_next = false;

        match arg {
            "--seed" | "--snapshot-seed" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<u64>() {
                        seed = v;
                    }
                    consume_next = true;
                }
            }
            "--qa-seed" | "--benchmark-seed" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<u64>() {
                        seed = v;
                    }
                    consume_next = true;
                }
            }
            "--snapshot-entities" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<usize>() {
                        initial_entities = v;
                    }
                    consume_next = true;
                }
            }
            "--qa-entities" | "--benchmark-entities" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<usize>() {
                        initial_entities = v;
                    }
                    consume_next = true;
                }
            }
            "--snapshot-ticks" => {
                if i + 1 < args.len() {
                    let parsed = parse_ticks_csv(&args[i + 1]);
                    if !parsed.is_empty() {
                        snapshot_ticks = parsed;
                    }
                    consume_next = true;
                }
            }
            "--snapshot-out" => {
                if i + 1 < args.len() {
                    snapshot_out = PathBuf::from(&args[i + 1]);
                    consume_next = true;
                }
            }
            "--snapshot" => {
                snapshot_enabled = true;
            }
            "--qa-verify" => {
                qa_enabled = true;
            }
            "--benchmark" => {
                benchmark_enabled = true;
            }
            "--snapshot-show-ui" => {
                snapshot_hide_ui = false;
            }
            "--snapshot-hide-ui" => {
                snapshot_hide_ui = true;
            }
            "--no-bloom" | "--snapshot-no-bloom" => {
                disable_bloom = true;
            }
            "--fx-quality" => {
                if i + 1 < args.len() {
                    if let Some(q) = VisualQuality::parse_cli(&args[i + 1]) {
                        visual.set_quality_preset(q);
                    }
                    consume_next = true;
                }
            }
            "--benchmark-quality" => {
                if i + 1 < args.len() {
                    if let Some(q) = VisualQuality::parse_cli(&args[i + 1]) {
                        visual.set_quality_preset(q);
                    }
                    consume_next = true;
                }
            }
            "--qa-scenario" => {
                if i + 1 < args.len() {
                    if let Some(scenario) = QaScenario::parse_cli(&args[i + 1]) {
                        qa_scenario = scenario;
                    }
                    consume_next = true;
                }
            }
            "--qa-out" => {
                if i + 1 < args.len() {
                    qa_out = PathBuf::from(&args[i + 1]);
                    consume_next = true;
                }
            }
            "--qa-show-ui" => {
                qa_show_ui = true;
            }
            "--qa-hide-ui" => {
                qa_show_ui = false;
            }
            "--benchmark-out" => {
                if i + 1 < args.len() {
                    benchmark_out = PathBuf::from(&args[i + 1]);
                    consume_next = true;
                }
            }
            "--benchmark-seconds" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<u32>() {
                        benchmark_run_seconds = v.max(1);
                    }
                    consume_next = true;
                }
            }
            "--benchmark-warmup-seconds" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<u32>() {
                        benchmark_warmup_seconds = v;
                    }
                    consume_next = true;
                }
            }
            "--benchmark-no-ui" => {
                benchmark_show_ui = false;
            }
            "--benchmark-show-ui" => {
                benchmark_show_ui = true;
            }
            "--no-atmosphere" => {
                visual.atmosphere_enabled = false;
            }
            "--no-storm-fx" => {
                visual.storm_fx_enabled = false;
            }
            "--no-creature-detail" => {
                visual.creature_detail_enabled = false;
            }
            "--no-trails" => {
                visual.trails_enabled = false;
            }
            "--auto-quality" => {
                auto_quality = true;
            }
            "--no-auto-quality" => {
                auto_quality = false;
            }
            "--quality-min" => {
                if i + 1 < args.len() {
                    if let Some(q) = VisualQuality::parse_cli(&args[i + 1]) {
                        quality_min = q;
                    }
                    consume_next = true;
                }
            }
            "--quality-max" => {
                if i + 1 < args.len() {
                    if let Some(q) = VisualQuality::parse_cli(&args[i + 1]) {
                        quality_max = q;
                    }
                    consume_next = true;
                }
            }
            "--speed-render-decimation" => {
                speed_render_decimation = true;
            }
            "--no-speed-render-decimation" => {
                speed_render_decimation = false;
            }
            "--max-sim-ms-per-frame" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<f64>() {
                        max_sim_ms_per_frame = v.max(1.0);
                    }
                    consume_next = true;
                }
            }
            "--render-every-n-frames" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<u32>() {
                        render_every_n_frames = Some(v.max(1));
                    }
                    consume_next = true;
                }
            }
            _ => {}
        }

        i += if consume_next { 2 } else { 1 };
    }

    let snapshot = if snapshot_enabled {
        Some(SnapshotSettings::new(
            snapshot_out,
            snapshot_ticks,
            snapshot_hide_ui,
        ))
    } else {
        None
    };
    let qa = if qa_enabled {
        Some(QaSettings {
            scenario: qa_scenario,
            output_dir: qa_out,
            show_ui: qa_show_ui,
        })
    } else {
        None
    };
    let benchmark = if benchmark_enabled {
        Some(BenchmarkSettings {
            output_dir: benchmark_out,
            run_seconds: benchmark_run_seconds,
            warmup_seconds: benchmark_warmup_seconds,
            show_ui: benchmark_show_ui,
        })
    } else {
        None
    };

    RunConfig {
        seed,
        initial_entities,
        snapshot,
        qa,
        benchmark,
        disable_bloom,
        visual,
        auto_quality,
        quality_bounds: VisualQualityBounds::new(quality_min, quality_max),
        speed_render_decimation,
        max_sim_ms_per_frame,
        render_every_n_frames,
    }
}

fn parse_run_config() -> RunConfig {
    let args: Vec<String> = std::env::args().collect();
    parse_run_config_from_args(&args)
}

fn snapshot_sample(sim: &SimState) -> SnapshotSample {
    let (avg_energy, avg_generation, avg_age, avg_size) = compute_population_summary(sim);
    SnapshotSample {
        tick: sim.tick_count,
        population: sim.arena.count,
        food: sim.food.len(),
        meat: sim.meat.len(),
        avg_energy,
        avg_age,
        avg_generation,
        avg_size,
        species_estimate: sim.cached_species_estimate,
        storm_active: sim.environment.storm.is_some(),
        wall_count: sim.environment.walls.len(),
        toxic_zone_count: sim.environment.toxic_zones.len(),
        season: sim.environment.season.name().to_string(),
        is_day: sim.environment.is_day(),
    }
}

fn record_stats(sim: &SimState, sim_stats: &mut SimStats) {
    sim_stats.record(
        sim.arena.count,
        sim.cached_avg_energy,
        sim.food.len(),
        sim.cached_avg_generation,
        sim.births_last_tick,
        sim.deaths_last_tick,
    );
}

fn write_pretty_json<T: Serialize>(path: &std::path::Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| format!("serialize {} failed: {e}", path.display()))?;
    std::fs::write(path, json).map_err(|e| format!("write {} failed: {e}", path.display()))
}

fn write_qa_outputs(
    director: &mut QaDirector,
    seed: u64,
    initial_entities: usize,
    final_tick: u64,
    visual: VisualSettings,
) -> Result<(), String> {
    director.finalize_behavior_checks();
    let report = director.report(seed, initial_entities, final_tick, visual);
    let report_path = director.output_dir().join("qa_report.json");
    write_pretty_json(&report_path, &report)?;

    let passing = report.checks.iter().filter(|c| c.passed).count();
    let summary_path = director.output_dir().join("qa_summary.txt");
    let summary = format!(
        "scenario={}\nstatus={}\nseed={}\ninitial_entities={}\nfinal_tick={}\nactions={}\ncaptures={}\nchecks_passed={}/{}\nvisual_quality={}\n",
        report.scenario,
        report.overall_status,
        report.seed,
        report.initial_entities,
        report.final_tick,
        report.action_count,
        report.capture_count,
        passing,
        report.checks.len(),
        report.visual_quality,
    );
    std::fs::write(&summary_path, summary)
        .map_err(|e| format!("write {} failed: {e}", summary_path.display()))
}

fn write_benchmark_outputs(
    samples: &[BenchmarkFrameSample],
    settings: &BenchmarkSettings,
    seed: u64,
    initial_entities: usize,
    sim: &SimState,
    visual: VisualSettings,
    measured_seconds: f64,
    measured_tick_total: u64,
    auto_quality: bool,
    quality_bounds: VisualQualityBounds,
    speed_render_decimation: bool,
    max_sim_ms_per_frame: f64,
) -> Result<(), String> {
    std::fs::create_dir_all(&settings.output_dir).map_err(|e| {
        format!(
            "create benchmark output dir {} failed: {e}",
            settings.output_dir.display()
        )
    })?;

    let mut frame_ms = MetricAggregator::new();
    let mut fps = MetricAggregator::new();
    let mut ticks_per_frame = MetricAggregator::new();
    for s in samples {
        frame_ms.push(s.frame_ms);
        fps.push(s.fps);
        ticks_per_frame.push(s.ticks_this_frame as f64);
    }

    let frame_ms_summary = frame_ms.summary();
    let fps_summary = fps.summary();
    let ticks_summary = ticks_per_frame.summary();
    let frames_under_16_7ms_pct = frame_ms.pct_leq(16.7);
    let criteria = BenchmarkPassCriteria {
        target_entities_ok: initial_entities >= 200,
        avg_fps_ok: fps_summary.mean >= 60.0,
        p95_frame_ms_ok: frame_ms_summary.p95 <= 16.7,
        overall: initial_entities >= 200
            && fps_summary.mean >= 60.0
            && frame_ms_summary.p95 <= 16.7,
    };
    let report = BenchmarkReport {
        meta: BenchmarkRunMeta {
            seed,
            initial_entities,
            run_seconds: settings.run_seconds,
            warmup_seconds: settings.warmup_seconds,
            quality: visual.quality.label().to_string(),
            show_ui: settings.show_ui,
            atmosphere: visual.atmosphere_enabled,
            storm_fx: visual.storm_fx_enabled,
            creature_detail: visual.creature_detail_enabled,
            trails: visual.trails_enabled,
            shelter_highlight: visual.shelter_highlight_enabled,
            auto_quality,
            quality_min: quality_bounds.min.label().to_string(),
            quality_max: quality_bounds.max.label().to_string(),
            speed_render_decimation,
            max_sim_ms_per_frame,
        },
        measured_frames: samples.len(),
        total_sim_ticks: sim.tick_count,
        avg_ticks_per_second: if measured_seconds > 0.0 {
            measured_tick_total as f64 / measured_seconds
        } else {
            0.0
        },
        frame_ms: frame_ms_summary,
        fps: fps_summary,
        ticks_per_frame: ticks_summary,
        frames_under_16_7ms_pct,
        criteria,
    };

    let json_path = settings.output_dir.join("benchmark_report.json");
    write_pretty_json(&json_path, &report)?;

    let csv_path = settings.output_dir.join("benchmark_report.csv");
    let mut csv =
        String::from("frame_idx,elapsed_sec,sim_tick,ticks_this_frame,frame_ms,fps,population\n");
    for s in samples {
        csv.push_str(&format!(
            "{},{:.6},{},{},{:.4},{:.3},{}\n",
            s.frame_idx,
            s.elapsed_sec,
            s.sim_tick,
            s.ticks_this_frame,
            s.frame_ms,
            s.fps,
            s.population
        ));
    }
    std::fs::write(&csv_path, csv)
        .map_err(|e| format!("write {} failed: {e}", csv_path.display()))?;

    let summary_path = settings.output_dir.join("benchmark_summary.txt");
    let summary = format!(
        "status={}\nseed={}\ninitial_entities={}\nrun_seconds={}\nwarmup_seconds={}\nmeasured_frames={}\navg_fps={:.3}\np95_frame_ms={:.3}\nframes_under_16_7ms_pct={:.2}\navg_ticks_per_second={:.3}\nquality={}\nauto_quality={}\nquality_min={}\nquality_max={}\nrender_decimation={}\nmax_sim_ms_per_frame={:.2}\ncriteria_target_entities_ok={}\ncriteria_avg_fps_ok={}\ncriteria_p95_frame_ms_ok={}\n",
        if report.criteria.overall { "PASS" } else { "FAIL" },
        seed,
        initial_entities,
        settings.run_seconds,
        settings.warmup_seconds,
        report.measured_frames,
        report.fps.mean,
        report.frame_ms.p95,
        report.frames_under_16_7ms_pct,
        report.avg_ticks_per_second,
        report.meta.quality,
        report.meta.auto_quality,
        report.meta.quality_min,
        report.meta.quality_max,
        report.meta.speed_render_decimation,
        report.meta.max_sim_ms_per_frame,
        report.criteria.target_entities_ok,
        report.criteria.avg_fps_ok,
        report.criteria.p95_frame_ms_ok,
    );
    std::fs::write(&summary_path, summary)
        .map_err(|e| format!("write {} failed: {e}", summary_path.display()))
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut run_cfg = parse_run_config();
    let run_seed = run_cfg.seed;
    let run_initial_entities = run_cfg.initial_entities;
    let mut visual_settings = run_cfg.visual;
    visual_settings.quality = run_cfg.quality_bounds.clamp(visual_settings.quality);
    let speed_policy = SpeedPolicy::new();
    let mut quality_controller = AdaptiveQualityController::new(
        run_cfg.quality_bounds,
        AdaptiveQualityConfig::default(),
    );

    let mut sim = SimState::new(run_cfg.initial_entities, run_cfg.seed);
    let mut camera = CameraController::new(sim.world.center());
    let mut accumulator = 0.0f64;
    let mut sim_stats = SimStats::new(1000);
    let mut ui_state = UiState::default();
    ui_state.set_runtime_quality_policy(
        run_cfg.auto_quality,
        run_cfg.speed_render_decimation,
        run_cfg.quality_bounds.min,
        run_cfg.quality_bounds.max,
        run_cfg.max_sim_ms_per_frame,
    );
    let mut bloom = if run_cfg.disable_bloom {
        None
    } else {
        post_processing::BloomPipeline::new()
    };
    let mut autosave_timer = 0.0f64;
    let qa_show_ui = run_cfg.qa.as_ref().map(|s| s.show_ui).unwrap_or(false);
    let mut qa_director = match run_cfg.qa.as_ref() {
        Some(settings) => match QaDirector::new(settings.scenario, settings.output_dir.clone()) {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("[GENESIS] QA director init failed: {e}");
                return;
            }
        },
        None => None,
    };
    let mut benchmark_elapsed = 0.0f64;
    let mut benchmark_measured_elapsed = 0.0f64;
    let mut benchmark_frame_idx = 0u64;
    let mut benchmark_samples: Vec<BenchmarkFrameSample> = Vec::new();
    let mut benchmark_measured_ticks = 0u64;
    let mut render_frame_idx = 0u64;

    if let Some(snapshot) = run_cfg.snapshot.as_ref() {
        eprintln!(
            "[GENESIS] Snapshot mode enabled: seed={}, entities={}, ticks={:?}, out={}, ui={}, quality={}",
            run_seed,
            run_initial_entities,
            snapshot.capture_ticks,
            snapshot.output_dir.display(),
            if snapshot.hide_ui { "hidden" } else { "shown" },
            visual_settings.quality.label(),
        );
    }
    if let Some(qa) = run_cfg.qa.as_ref() {
        eprintln!(
            "[GENESIS] QA mode enabled: scenario={}, seed={}, entities={}, out={}, ui={}",
            qa.scenario.label(),
            run_seed,
            run_initial_entities,
            qa.output_dir.display(),
            if qa.show_ui { "shown" } else { "hidden" }
        );
    }
    if let Some(bench) = run_cfg.benchmark.as_ref() {
        eprintln!(
            "[GENESIS] Benchmark mode enabled: seed={}, entities={}, run={}s warmup={}s out={} ui={} quality={} auto_quality={} qrange={}..{}",
            run_seed,
            run_initial_entities,
            bench.run_seconds,
            bench.warmup_seconds,
            bench.output_dir.display(),
            if bench.show_ui { "shown" } else { "hidden" },
            visual_settings.quality.label(),
            run_cfg.auto_quality,
            run_cfg.quality_bounds.min.label(),
            run_cfg.quality_bounds.max.label(),
        );
    }

    loop {
        if let Some(ref mut director) = qa_director {
            // Deterministic QA runner: apply scripted actions -> render -> capture -> tick.
            camera.update(&sim.arena, config::FIXED_DT);
            director.run_actions_for_current_tick(
                &mut sim,
                &mut camera,
                &mut ui_state,
                &mut visual_settings,
            );
            let qa_alpha = director.render_alpha_override(sim.tick_count);

            if let Some(ref mut b) = bloom {
                b.check_resize();
                renderer::draw_with_bloom(&sim, &camera, qa_alpha, b, visual_settings);
            } else {
                renderer::draw(&sim, &camera, qa_alpha, visual_settings);
            }

            if qa_show_ui {
                ui::draw_ui(
                    &mut sim,
                    &mut camera,
                    &mut ui_state,
                    &sim_stats,
                    &mut visual_settings,
                );
            }

            if let Err(e) = director.capture_pending_frames() {
                eprintln!("[GENESIS] QA frame capture failed: {e}");
                break;
            }

            if director.is_complete(sim.tick_count) {
                if let Err(e) = write_qa_outputs(
                    director,
                    run_seed,
                    run_initial_entities,
                    sim.tick_count,
                    visual_settings,
                ) {
                    eprintln!("[GENESIS] QA report write failed: {e}");
                } else {
                    eprintln!(
                        "[GENESIS] QA verification complete. Output: {}",
                        director.output_dir().display()
                    );
                }
                break;
            }

            sim.tick();
            director.observe_state(&sim);
            record_stats(&sim, &mut sim_stats);
            next_frame().await;
            continue;
        }

        if let Some(ref benchmark) = run_cfg.benchmark {
            // Benchmark mode uses the real render+sim loop with warmup and fixed-duration measurement.
            let frame_time = get_frame_time() as f64;
            let frame_ms = (frame_time * 1000.0) as f32;
            benchmark_elapsed += frame_time;
            accumulator += frame_time.min(0.1);

            let target_hz = speed_policy.target_sim_hz(1.0);
            let effective_dt = 1.0 / target_hz;
            let mut ticks_this_frame = 0u32;
            let sim_budget_start = get_time();
            let mut budget_exhausted = false;
            while accumulator >= effective_dt {
                if (get_time() - sim_budget_start) * 1000.0 >= ui_state.max_sim_ms_per_frame {
                    budget_exhausted = true;
                    break;
                }
                sim.tick();
                record_stats(&sim, &mut sim_stats);
                accumulator -= effective_dt;
                ticks_this_frame += 1;
            }
            if budget_exhausted {
                accumulator = clamp_sim_accumulator(accumulator, effective_dt);
            }
            camera.update(&sim.arena, frame_time as f32);

            let runtime_bounds = VisualQualityBounds::new(ui_state.quality_min, ui_state.quality_max);
            if ui_state.auto_quality_enabled {
                quality_controller.set_bounds(runtime_bounds);
                let q =
                    quality_controller.observe(visual_settings.quality, frame_ms, 1000.0 / 60.0);
                visual_settings.set_quality_only(q);
            } else {
                visual_settings.set_quality_only(runtime_bounds.clamp(visual_settings.quality));
            }
            let render_every_n = run_cfg.render_every_n_frames.unwrap_or_else(|| {
                speed_policy.render_every_n_frames(1.0, ui_state.speed_render_decimation)
            });
            ui_state.set_perf_snapshot(
                visual_settings.quality,
                target_hz as f32,
                render_every_n,
                ui_state.auto_quality_enabled,
                frame_ms,
                ticks_this_frame,
            );

            if let Some(ref mut b) = bloom {
                b.check_resize();
                renderer::draw_with_bloom(&sim, &camera, 1.0, b, visual_settings);
            } else {
                renderer::draw(&sim, &camera, 1.0, visual_settings);
            }
            if benchmark.show_ui {
                ui::draw_ui(
                    &mut sim,
                    &mut camera,
                    &mut ui_state,
                    &sim_stats,
                    &mut visual_settings,
                );
            }

            if benchmark_elapsed >= benchmark.warmup_seconds as f64 {
                let fps = if frame_time > 0.000001 {
                    1.0 / frame_time
                } else {
                    0.0
                };
                benchmark_measured_elapsed += frame_time;
                benchmark_measured_ticks += ticks_this_frame as u64;
                benchmark_samples.push(BenchmarkFrameSample {
                    frame_idx: benchmark_frame_idx,
                    elapsed_sec: benchmark_elapsed,
                    sim_tick: sim.tick_count,
                    ticks_this_frame,
                    frame_ms: frame_ms as f64,
                    fps,
                    population: sim.arena.count,
                });
            }

            if benchmark_elapsed >= (benchmark.warmup_seconds as f64 + benchmark.run_seconds as f64)
            {
                if let Err(e) = write_benchmark_outputs(
                    &benchmark_samples,
                    benchmark,
                    run_seed,
                    run_initial_entities,
                    &sim,
                    visual_settings,
                    benchmark_measured_elapsed,
                    benchmark_measured_ticks,
                    ui_state.auto_quality_enabled,
                    VisualQualityBounds::new(ui_state.quality_min, ui_state.quality_max),
                    ui_state.speed_render_decimation,
                    ui_state.max_sim_ms_per_frame,
                ) {
                    eprintln!("[GENESIS] Benchmark report write failed: {e}");
                } else {
                    eprintln!(
                        "[GENESIS] Benchmark complete. Output: {}",
                        benchmark.output_dir.display()
                    );
                }
                break;
            }

            benchmark_frame_idx += 1;
            next_frame().await;
            continue;
        }

        if let Some(snapshot) = run_cfg.snapshot.as_mut() {
            // Deterministic snapshot runner: render -> capture -> tick.
            camera.update(&sim.arena, config::FIXED_DT);

            if let Some(ref mut b) = bloom {
                b.check_resize();
                renderer::draw_with_bloom(&sim, &camera, 1.0, b, visual_settings);
            } else {
                renderer::draw(&sim, &camera, 1.0, visual_settings);
            }

            if !snapshot.hide_ui {
                ui::draw_ui(
                    &mut sim,
                    &mut camera,
                    &mut ui_state,
                    &sim_stats,
                    &mut visual_settings,
                );
            }

            snapshot.capture_current_frame(&sim);

            if snapshot.is_complete(sim.tick_count) {
                if let Err(e) =
                    snapshot.write_report(&sim, run_seed, run_initial_entities, visual_settings)
                {
                    eprintln!("[GENESIS] Snapshot report write failed: {e}");
                } else {
                    eprintln!(
                        "[GENESIS] Snapshot run complete. Output: {}",
                        snapshot.output_dir.display()
                    );
                }
                break;
            }

            sim.tick();
            record_stats(&sim, &mut sim_stats);

            next_frame().await;
            continue;
        }

        let frame_time = get_frame_time() as f64;
        accumulator += frame_time.min(0.1);

        // Autosave timer
        if !sim.paused {
            autosave_timer += frame_time;
            if autosave_timer >= AUTOSAVE_INTERVAL {
                autosave_timer = 0.0;
                match save_load::save_to_file(&sim, "genesis_autosave.bin") {
                    Ok(()) => eprintln!(
                        "[GENESIS] Autosaved to genesis_autosave.bin (tick {})",
                        sim.tick_count
                    ),
                    Err(e) => eprintln!("[GENESIS] Autosave failed: {e}"),
                }
            }
        }

        let frame_ms = (frame_time * 1000.0) as f32;
        let target_hz = speed_policy.target_sim_hz(sim.speed_multiplier);
        let effective_dt = 1.0 / target_hz.max(1.0);
        accumulator = clamp_sim_accumulator(accumulator, effective_dt);
        let mut ticks_this_frame = 0u32;
        if !sim.paused {
            let sim_budget_start = get_time();
            let mut budget_exhausted = false;
            while accumulator >= effective_dt {
                if (get_time() - sim_budget_start) * 1000.0 >= ui_state.max_sim_ms_per_frame {
                    budget_exhausted = true;
                    break;
                }
                sim.tick();
                record_stats(&sim, &mut sim_stats);
                accumulator -= effective_dt;
                ticks_this_frame += 1;
            }
            if budget_exhausted {
                accumulator = clamp_sim_accumulator(accumulator, effective_dt);
            }
        } else {
            if ui_state.step_requested {
                sim.tick();
                record_stats(&sim, &mut sim_stats);
                ui_state.step_requested = false;
                ticks_this_frame = 1;
            }
            accumulator = 0.0;
        }

        camera.update(&sim.arena, get_frame_time());

        let speed_cap = speed_policy.quality_cap_for_speed(sim.speed_multiplier);
        let requested_bounds = VisualQualityBounds::new(ui_state.quality_min, ui_state.quality_max);
        let capped_max =
            VisualQuality::from_rank(requested_bounds.max.rank().min(speed_cap.rank()));
        let runtime_bounds = VisualQualityBounds::new(requested_bounds.min, capped_max);
        quality_controller.set_bounds(runtime_bounds);
        if ui_state.auto_quality_enabled {
            let adjusted =
                quality_controller.observe(visual_settings.quality, frame_ms, 1000.0 / 60.0);
            visual_settings.set_quality_only(adjusted);
        } else {
            visual_settings.set_quality_only(runtime_bounds.clamp(visual_settings.quality));
        }
        let render_every_n = run_cfg.render_every_n_frames.unwrap_or_else(|| {
            speed_policy.render_every_n_frames(sim.speed_multiplier, ui_state.speed_render_decimation)
        });
        ui_state.set_perf_snapshot(
            visual_settings.quality,
            target_hz as f32,
            render_every_n,
            ui_state.auto_quality_enabled,
            frame_ms,
            ticks_this_frame,
        );

        // World interaction tools (only if egui doesn't want pointer input)
        let mut egui_wants_pointer = false;
        egui_macroquad::cfg(|ctx| {
            egui_wants_pointer = ctx.wants_pointer_input();
        });
        if !egui_wants_pointer {
            let mouse_screen = Vec2::from(mouse_position());
            let mouse_world = camera.screen_to_world(mouse_screen);

            match ui_state.tool_mode {
                ToolMode::Select => {
                    if is_mouse_button_pressed(MouseButton::Left) {
                        let pick_radius = 30.0 / camera.smooth_zoom;
                        if let Some(id) = camera.pick_entity(mouse_world, &sim.arena, pick_radius) {
                            camera.following = Some(id);
                        } else {
                            camera.following = None;
                        }
                    }
                }
                ToolMode::SpawnFood => {
                    if is_mouse_button_pressed(MouseButton::Left) {
                        sim.spawn_food_cluster(mouse_world, config::FOOD_CLUSTER_COUNT);
                    }
                }
                ToolMode::SpawnHazard => {
                    if is_mouse_button_pressed(MouseButton::Left)
                        || is_mouse_button_pressed(MouseButton::Right)
                    {
                        sim.spawn_toxic_zone(mouse_world);
                    }
                }
                ToolMode::DrawWall => {
                    if is_mouse_button_pressed(MouseButton::Left) {
                        ui_state.wall_drag_start = Some(mouse_world);
                    }
                    if is_mouse_button_released(MouseButton::Left) {
                        if let Some(start) = ui_state.wall_drag_start.take() {
                            sim.add_wall(start, mouse_world);
                        }
                    }
                }
            }

            // Prompt-compatible shortcut: right-click always places temporary hazard.
            if is_mouse_button_pressed(MouseButton::Right) {
                sim.spawn_toxic_zone(mouse_world);
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

        // Spawn random entity at cursor
        if is_key_pressed(KeyCode::E) {
            let mouse_world = camera.screen_to_world(Vec2::from(mouse_position()));
            let genome = genome::Genome::random(&mut sim.rng);
            let entity = entity::Entity::new_from_genome_rng(
                &genome,
                sim.world.wrap(mouse_world),
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
                        eprintln!(
                            "[GENESIS] Loaded from genesis_save.bin (tick {})",
                            sim.tick_count
                        );
                    }
                    Err(e) => eprintln!("[GENESIS] Load failed: {e}"),
                }
            }
        }

        let alpha = compute_render_alpha(accumulator, effective_dt, sim.paused);

        let should_render_world = sim.paused || render_frame_idx % render_every_n as u64 == 0;
        if should_render_world {
            // Render scene (with or without bloom)
            if let Some(ref mut b) = bloom {
                b.check_resize();
                renderer::draw_with_bloom(&sim, &camera, alpha, b, visual_settings);
            } else {
                renderer::draw(&sim, &camera, alpha, visual_settings);
            }
        }
        render_frame_idx = render_frame_idx.wrapping_add(1);

        // Draw egui UI on top
        ui::draw_ui(
            &mut sim,
            &mut camera,
            &mut ui_state,
            &sim_stats,
            &mut visual_settings,
        );

        next_frame().await;
    }
}

fn compute_population_summary(sim: &SimState) -> (f32, f32, f32, f32) {
    let mut total_energy = 0.0f32;
    let mut total_gen = 0.0f32;
    let mut total_age = 0.0f32;
    let mut total_size = 0.0f32;
    let mut count = 0u32;

    for (_idx, e) in sim.arena.iter_alive() {
        total_energy += e.energy;
        total_gen += e.generation_depth as f32;
        total_age += e.age;
        total_size += e.radius / config::ENTITY_BASE_RADIUS;
        count += 1;
    }

    if count > 0 {
        (
            total_energy / count as f32,
            total_gen / count as f32,
            total_age / count as f32,
            total_size / count as f32,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    }
}

fn compute_render_alpha(accumulator: f64, effective_dt: f64, paused: bool) -> f32 {
    if paused {
        return 1.0;
    }
    if effective_dt <= f64::EPSILON {
        return 1.0;
    }
    (accumulator / effective_dt).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &[&str]) -> Vec<String> {
        input.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_run_config_visual_flags() {
        let cfg = parse_run_config_from_args(&args(&[
            "genesis",
            "--fx-quality",
            "ultra",
            "--no-atmosphere",
            "--no-storm-fx",
            "--no-creature-detail",
            "--no-trails",
            "--snapshot",
            "--snapshot-entities",
            "120",
            "--seed",
            "77",
        ]));

        assert_eq!(cfg.seed, 77);
        assert_eq!(cfg.initial_entities, 120);
        assert!(cfg.snapshot.is_some());
        assert_eq!(cfg.visual.quality, VisualQuality::Ultra);
        assert!(!cfg.visual.atmosphere_enabled);
        assert!(!cfg.visual.storm_fx_enabled);
        assert!(!cfg.visual.creature_detail_enabled);
        assert!(!cfg.visual.trails_enabled);
    }

    #[test]
    fn parse_run_config_defaults_to_high_quality() {
        let cfg = parse_run_config_from_args(&args(&["genesis"]));
        assert_eq!(cfg.visual.quality, VisualQuality::High);
        assert!(cfg.visual.atmosphere_enabled);
        assert!(cfg.visual.storm_fx_enabled);
        assert!(cfg.auto_quality);
        assert!(cfg.speed_render_decimation);
        assert_eq!(cfg.quality_bounds.min, VisualQuality::Low);
        assert_eq!(cfg.quality_bounds.max, VisualQuality::Ultra);
        assert!(cfg.qa.is_none());
        assert!(cfg.benchmark.is_none());
    }

    #[test]
    fn parse_run_config_qa_flags() {
        let cfg = parse_run_config_from_args(&args(&[
            "genesis",
            "--qa-verify",
            "--qa-scenario",
            "baseline",
            "--qa-seed",
            "123",
            "--qa-entities",
            "88",
            "--qa-out",
            "qa_runs/test",
            "--qa-hide-ui",
        ]));

        assert_eq!(cfg.seed, 123);
        assert_eq!(cfg.initial_entities, 88);
        let qa = cfg.qa.expect("qa should be enabled");
        assert_eq!(qa.scenario, QaScenario::Baseline);
        assert_eq!(qa.output_dir, PathBuf::from("qa_runs/test"));
        assert!(!qa.show_ui);
    }

    #[test]
    fn parse_run_config_boundary_probe_scenario() {
        let cfg = parse_run_config_from_args(&args(&[
            "genesis",
            "--qa-verify",
            "--qa-scenario",
            "boundary-probe",
        ]));
        let qa = cfg.qa.expect("qa should be enabled");
        assert_eq!(qa.scenario, QaScenario::BoundaryProbe);
    }

    #[test]
    fn parse_run_config_benchmark_flags() {
        let cfg = parse_run_config_from_args(&args(&[
            "genesis",
            "--benchmark",
            "--benchmark-seed",
            "555",
            "--benchmark-entities",
            "240",
            "--benchmark-seconds",
            "90",
            "--benchmark-warmup-seconds",
            "10",
            "--benchmark-out",
            "benchmark_runs/test",
            "--benchmark-quality",
            "medium",
            "--benchmark-show-ui",
        ]));

        assert_eq!(cfg.seed, 555);
        assert_eq!(cfg.initial_entities, 240);
        assert_eq!(cfg.visual.quality, VisualQuality::Medium);
        let bench = cfg.benchmark.expect("benchmark should be enabled");
        assert_eq!(bench.output_dir, PathBuf::from("benchmark_runs/test"));
        assert_eq!(bench.run_seconds, 90);
        assert_eq!(bench.warmup_seconds, 10);
        assert!(bench.show_ui);
    }

    #[test]
    fn parse_run_config_speed_quality_policy_flags() {
        let cfg = parse_run_config_from_args(&args(&[
            "genesis",
            "--no-auto-quality",
            "--quality-min",
            "medium",
            "--quality-max",
            "high",
            "--no-speed-render-decimation",
            "--max-sim-ms-per-frame",
            "12.5",
            "--render-every-n-frames",
            "3",
        ]));

        assert!(!cfg.auto_quality);
        assert!(!cfg.speed_render_decimation);
        assert!((cfg.max_sim_ms_per_frame - 12.5).abs() < 1e-6);
        assert_eq!(cfg.render_every_n_frames, Some(3));
        assert_eq!(cfg.quality_bounds.min, VisualQuality::Medium);
        assert_eq!(cfg.quality_bounds.max, VisualQuality::High);
    }

    #[test]
    fn speed_policy_avoids_decimation_at_two_x() {
        let policy = SpeedPolicy::new();
        assert_eq!(policy.render_every_n_frames(1.0, true), 1);
        assert_eq!(policy.render_every_n_frames(2.0, true), 1);
        assert_eq!(policy.render_every_n_frames(5.0, true), 2);
        assert_eq!(policy.render_every_n_frames(10.0, true), 3);
    }

    #[test]
    fn sim_accumulator_is_clamped_to_prevent_backlog_spiral() {
        let dt = 1.0 / 120.0;
        let capped = clamp_sim_accumulator(0.5, dt);
        assert!(capped <= dt * MAX_SIM_ACCUMULATOR_STEPS + 1e-9);
        assert_eq!(clamp_sim_accumulator(0.1, 0.0), 0.0);
    }

    #[test]
    fn render_alpha_is_clamped_for_interactive_loop() {
        assert!((compute_render_alpha(0.02, 0.04, false) - 0.5).abs() < 1e-6);
        assert!((compute_render_alpha(0.2, 0.04, false) - 1.0).abs() < 1e-6);
        assert!((compute_render_alpha(-0.1, 0.04, false) - 0.0).abs() < 1e-6);
        assert!((compute_render_alpha(0.2, 0.0, false) - 1.0).abs() < 1e-6);
        assert!((compute_render_alpha(0.2, 0.04, true) - 1.0).abs() < 1e-6);
    }
}
