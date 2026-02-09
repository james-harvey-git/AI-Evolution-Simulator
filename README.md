# GENESIS — Neural Evolution Simulator

A real-time artificial life simulation where CTRNN-brained entities evolve via natural selection in a rich 2D environment. All behavior — foraging, predation, cooperation, signalling, and migration — emerges from evolutionary pressure, not hardcoded logic.

## Features

- **CTRNN Brains**: Each entity has a Continuous-Time Recurrent Neural Network with fixed sensor/motor channels and evolvable interneuron topology
- **Raycast + Internal Perception**: 8-ray local sensing plus proprioceptive inputs (energy, health, age, speed, carried resources, adjacent contact, pheromone alignment)
- **Natural Selection**: No fitness function — entities that gather energy survive and reproduce, those that don't die
- **Genetic Inheritance**: Offspring receive mutated copies of parent genomes including neural weights, body size, speed, color, sensor range, metabolic rate, evolvable mutation rate, and evolvable mutation magnitude
- **Combat & Predation**: Entities can attack others with damage scaling by size and current energy, dropping meat on kills
- **Resource Handling**: Entities can eat immediately or pick up resources for later digestion
- **Chemical Signalling**: RGB signal broadcasting and pheromone trails
- **Rich Environment**: Perlin-noise terrain (Plains, Forest, Desert, Water, Toxic), day/night cycles, seasons, roaming storms, temporary toxic zones, user-drawn walls, and wall-proximity storm shelter
- **Storm Atmosphere FX**: Directional wind-line storms, gust arcs, and wall-shelter highlight bands during active storms
- **Visual Quality Presets**: Runtime/CLI quality controls (`Low`/`Medium`/`High`/`Ultra`) with granular FX toggles for atmosphere, trails, and creature detail
- **Adaptive Performance Policy**: Auto quality scaling with hysteresis, sim-time budget guardrails, and speed-based render decimation for stable high-speed runs
- **Bloom + Grading Post-Processing**: GPU-accelerated glow with quality-scaled threshold/intensity plus lightweight grading/vignette
- **Particle Effects**: Visual feedback for births, deaths, eating, and combat
- **World-First UI Shell**: Slim status strip + right-side tab dock (Inspector/Brain/Graphs/Settings) with performance badges
- **Autonomous QA Probes**: Deterministic visual scenarios with pass/fail checks for panel behavior, storm state, world bounds, finite positions, and rapid-turn stability
- **Save/Load**: Binary serialization of full simulation state (Ctrl+S / Ctrl+L)

## Building

Requires Rust 1.70+.

```bash
cargo run --release
```

## Testing

Run the unit test suite:

```bash
cargo test
```

## Deterministic Snapshot Verification

Run a reproducible visual capture pass with fixed seed/ticks:

```bash
cargo run --release -- \
  --snapshot \
  --snapshot-seed 42 \
  --snapshot-entities 80 \
  --snapshot-ticks 0,300,1000,3000,5000 \
  --snapshot-out snapshot_runs/latest
```

Outputs:

- PNG frames: `frame_XXX_tick_YYYYYY.png`
- `snapshot_report.csv` with per-capture metrics
- `summary.txt` with final run metrics

Useful flags:

- `--snapshot-show-ui` to include UI overlays in captures (default hides UI)
- `--snapshot-no-bloom` to disable bloom during snapshot runs
- `--fx-quality low|medium|high|ultra` to select visual preset (default: `high`)
- `--no-atmosphere` to disable atmosphere overlays
- `--no-storm-fx` to disable directional storm wind lines/gusts
- `--no-creature-detail` to disable high-detail creature shading/animation
- `--no-trails` to disable creature movement trails
- `--auto-quality` / `--no-auto-quality` to enable or disable adaptive quality scaling
- `--quality-min low|medium|high|ultra` and `--quality-max ...` to constrain adaptive quality range
- `--max-sim-ms-per-frame <ms>` to cap simulation time budget per rendered frame
- `--speed-render-decimation` / `--no-speed-render-decimation` to toggle automatic render skipping at high speed
- `--render-every-n-frames <n>` to force a fixed render decimation rate

## Autonomous QA Verification

Run scripted, deterministic UI + visual verification with zero manual clicks:

```bash
cargo run --release -- \
  --qa-verify \
  --qa-scenario baseline \
  --qa-seed 42 \
  --qa-entities 120 \
  --qa-out qa_runs/latest \
  --qa-show-ui
```

Outputs:

- QA frames: `qa_frames/frame_XXX_<label>.png`
- `qa_report.json` with actions, checks, and pass/fail status
- `qa_summary.txt` with high-level result

Useful flags:

- `--qa-hide-ui` to render without egui overlays
- `--qa-scenario boundary-probe` to stress interpolation/render paths (alpha stress + bounds checks)
- `--fx-quality ...` and visual toggles also apply during QA runs

## Performance Benchmark Evidence

Run a reproducible perf evidence pass (real render + simulation loop):

```bash
cargo run --release -- \
  --benchmark \
  --benchmark-seed 42 \
  --benchmark-entities 220 \
  --benchmark-seconds 60 \
  --benchmark-warmup-seconds 5 \
  --benchmark-out benchmark_runs/latest \
  --benchmark-quality high \
  --benchmark-no-ui
```

Outputs:

- `benchmark_report.json` (machine-readable metrics + criteria checks)
- `benchmark_report.csv` (per-frame samples)
- `benchmark_summary.txt` (human-readable pass/fail summary)

Benchmark metadata now includes:

- visual quality mode and adaptive policy (`auto_quality`, `quality_min`, `quality_max`)
- render decimation state
- simulation budget cap (`max_sim_ms_per_frame`)

Key benchmark criteria tracked:

- `target_entities >= 200`
- `avg_fps >= 60`
- `p95_frame_ms <= 16.7`
- `%frames_under_16_7ms`

## Controls

| Key / Mouse | Action |
|---|---|
| **WASD** / Arrow keys | Pan camera |
| **Scroll wheel** | Zoom in/out |
| **Left click** | Use selected tool (select entity / spawn food / spawn hazard / draw wall) |
| **Right click** | Spawn temporary toxic zone |
| **Middle mouse drag** | Pan camera |
| **Escape** | Deselect entity |
| **Space** | Pause / Resume |
| **E** | Spawn random entity at cursor |
| **Delete / Backspace** | Smite selected entity |
| **R** | Toggle sensor-ray overlay |
| **Ctrl+S** | Save simulation |
| **Ctrl+L** | Load simulation |

## UI Panels

- **Status Strip (top)**: Play/pause/step, speed controls, tool mode, panel toggles, hero metrics, and performance badges
- **Right Dock**: Tabbed Inspector / Brain / Graphs / Settings for cleaner world-first layout
- **Inspector**: Selected entity vitals, genome traits, lineage, and live brain outputs
- **Brain**: Real-time neural network visualization with dynamic topology labels
- **Graphs**: Population, energy, food, births/deaths, and generation trends
- **Minimap**: World overview with entities, storms, walls, hazards, and camera viewport
- **Settings**: Spawn helpers, storm trigger, visual FX toggles, and runtime performance policy controls

## Architecture

```
src/
  main.rs             Entry point, run modes, speed policy, QA/benchmark orchestration
  config.rs           All tunable constants
  world.rs            World bounds, toroidal wrapping
  entity.rs           Entity struct, generational arena
  brain.rs            CTRNN implementation (SoA layout)
  genome.rs           Genome encoding, mutation
  sensory.rs          Raycast perception system
  physics.rs          Movement, collision response
  spatial_hash.rs     Uniform grid spatial index
  energy.rs           Metabolism, food consumption, starvation
  reproduction.rs     Asexual reproduction, mutation pipeline
  combat.rs           Attack, damage, meat drops
  signals.rs          RGB signalling, pheromone grid
  environment.rs      Terrain, day/night, seasons, storms
  simulation.rs       Tick orchestration
  camera.rs           Pan, zoom, follow camera
  renderer.rs         All macroquad draw calls
  particles.rs        Particle system for visual effects
  post_processing.rs  Bloom + grading pipeline via render targets + shaders
  qa.rs               Deterministic autonomous QA scenario runner + report writer
  reporting.rs        Shared metrics/statistics summaries for reports
  visual.rs           Visual quality presets and runtime FX settings
  save_load.rs        Binary serialization via serde + bincode
  stats.rs            Rolling statistics ring buffers
  ui/                 egui shell (status strip + right dock + panel content)
```

## Technical Details

- **Engine**: macroquad 0.4 (OpenGL, Apple Silicon compatible)
- **Brain**: Forward Euler integration of CTRNN, 15 fixed sensor + 10 fixed motor neurons, with evolvable interneurons in [3, 16]
- **Genome**: Variable length based on topology (neural weights/biases/time constants + 9 body params)
- **Runtime**: Speed policy targets 60/120/300/600 sim Hz by speed tier with frame-budget throttling and decimated rendering at high speeds
- **Physics**: Fixed-step simulation with render interpolation, spatial hash for O(1) neighbor queries, and mass-aware entity collision separation
- **World**: 2000x2000 toroidal, Perlin-noise terrain generation

## License

MIT
