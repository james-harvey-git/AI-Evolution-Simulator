# GENESIS — Neural Evolution Simulator

A real-time artificial life simulation where CTRNN-brained entities evolve via natural selection in a rich 2D environment. All behavior — foraging, predation, cooperation, signalling, and migration — emerges from evolutionary pressure, not hardcoded logic.

## Features

- **CTRNN Brains**: Each entity has a 12-neuron Continuous-Time Recurrent Neural Network controlling movement, attack, and signalling
- **Raycast Perception**: 8-ray sensory system detecting food, entities, and environmental features
- **Natural Selection**: No fitness function — entities that gather energy survive and reproduce, those that don't die
- **Genetic Inheritance**: Offspring receive mutated copies of parent genomes including neural weights, body size, speed, color, and evolvable mutation rate
- **Combat & Predation**: Entities can attack others, dropping meat on kills
- **Chemical Signalling**: RGB signal broadcasting and pheromone trails
- **Rich Environment**: Perlin-noise terrain (Plains, Forest, Desert, Water, Toxic), day/night cycles, seasons, roaming storms
- **Bloom Post-Processing**: GPU-accelerated glow effects on signals and bright elements
- **Particle Effects**: Visual feedback for births, deaths, eating, and combat
- **Full UI**: egui-powered inspector, neural network visualizer, population graphs, minimap, and settings panel
- **Save/Load**: Binary serialization of full simulation state (Ctrl+S / Ctrl+L)

## Building

Requires Rust 1.70+.

```bash
cargo run --release
```

## Controls

| Key / Mouse | Action |
|---|---|
| **WASD** / Arrow keys | Pan camera |
| **Scroll wheel** | Zoom in/out |
| **Left click** | Select entity |
| **Middle mouse drag** | Pan camera |
| **Escape** | Deselect entity |
| **Space** | Pause / Resume |
| **Ctrl+S** | Save simulation |
| **Ctrl+L** | Load simulation |

## UI Panels

- **Toolbar** (top): Pause/play, speed control (0.25x–8x), entity/food/tick counts, panel toggles
- **Inspector** (left): Selected entity stats, genome traits, brain outputs, lineage info
- **Brain**: Real-time neural network visualization with activation colors and weight lines
- **Graphs**: Population, average energy, food count, births/deaths, average generation over time
- **Minimap**: World overview with entity dots, food, storms, and camera viewport
- **Settings**: Spawn tools (food, entities), trigger storms, system info

## Architecture

```
src/
  main.rs             Entry point, main loop, fixed timestep
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
  post_processing.rs  Bloom pipeline via render targets + shaders
  save_load.rs        Binary serialization via serde + bincode
  stats.rs            Rolling statistics ring buffers
  ui/                 egui panels (toolbar, inspector, neural_viz, graphs, minimap, settings)
```

## Technical Details

- **Engine**: macroquad 0.4 (OpenGL, Apple Silicon compatible)
- **Brain**: Forward Euler integration of CTRNN, 12 neurons (6 sensor, 2 interneuron, 4 motor), tau range [0.5, 5.0], weight scale [-16, 16]
- **Genome**: 176 floats (144 weights + 12 biases + 12 time constants + 8 body params)
- **Physics**: Fixed 60Hz timestep with render interpolation, spatial hash for O(1) neighbor queries
- **World**: 2000x2000 toroidal, Perlin-noise terrain generation

## License

MIT
