# Prompt Gap Analysis: GENESIS vs Initial Claude Prompt

Source prompt reviewed: `/Users/jamesharvey/Downloads/claude_code_prompt.md`.

Status labels:
- `Implemented`: present in code and integrated in main loop/UI.
- `Partial`: present but simplified or not matching prompt depth.
- `Missing`: not implemented yet.

## High-Level Summary

- Core simulation loop, evolution substrate, and most interaction hooks are in place.
- Open-endedness depth has materially improved: structural brain mutation, wall-based storm shelter, and mass/energy-sensitive combat-physics loops are implemented.
- Visual/runtime delivery has been overhauled: cinematic terrain/atmosphere pass, world-first docked UI shell, adaptive quality scaling, and reproducible performance evidence tooling are implemented.
- Remaining gaps are now mostly stretch-goal ecosystem/UI depth, not baseline capability.

## Requirement Matrix

| Prompt Requirement | Status | Notes / Evidence |
|---|---|---|
| CTRNN brain per entity | Implemented | `src/brain.rs`, `src/genome.rs` |
| Local sensing only (rays/cone, no omniscience) | Implemented | Raycast sensors in `src/sensory.rs` |
| 8-16 sensory rays with object typing | Implemented | 8 rays, hit types include entity/food/wall/hazard |
| Internal proprioception inputs (energy/health/age/speed/carrying) | Implemented | Sensor channels include these values |
| Mouth/adjacent interaction sensor | Implemented | `adjacent_contact` channel in `src/sensory.rs` |
| Continuous outputs: move/eat/attack/share/pickup/signal/reproduce | Implemented | `MotorOutputs` in `src/brain.rs` |
| Genome encodes neural + body traits | Implemented | `src/genome.rs` |
| Evolvable mutation rate and mutation magnitude | Implemented | `mutation_rate()` + `mutation_sigma()` |
| Structural mutation (add/remove CTRNN neuron) | Implemented | Dynamic interneuron topology with bounded add/remove mutation in `src/genome.rs`, `src/brain.rs`, `src/save_load.rs` |
| No explicit fitness function (natural selection) | Implemented | Energy/health/death/reproduction dynamics |
| Food regrowth + meat drops | Implemented | `src/energy.rs`, `src/combat.rs` |
| Seasonal abundance variation | Implemented | `Season::food_multiplier` + day/night gate |
| Toxic zones + storms hazards | Implemented | `src/environment.rs` |
| Terrain friction zones | Implemented | `TerrainType::friction_mult`, used in movement |
| Obstacles/walls block movement and LoS | Implemented | Wall collision + sensory ray wall hits |
| Shelter near walls/obstacles reduces storm effects | Implemented | Combined wall + forest shelter multiplier attenuates storm damage and wind push in `src/environment.rs` |
| Day/night visual change and day-only regrowth | Implemented | Overlay + night regrowth disabled |
| Pheromone trail + gradient sensing | Implemented | `src/signals.rs`, `src/sensory.rs` |
| Combat damage depends on size and energy | Implemented | Damage scales by size and attacker energy in `src/combat.rs` |
| Food sharing between adjacent entities | Implemented | `SimState::process_food_sharing` |
| RGB signalling visible to others | Implemented | Signal outputs + ray-mediated sensing |
| Reproduction trigger output + energy gating | Implemented | Intent threshold + energy threshold |
| Size tradeoff (larger slower and more costly) | Partial | Health/damage influenced by size; speed/metabolism are independent genes, no enforced size-energy tradeoff |
| Mass proportional to size in collisions | Implemented | Collision separation weights displacement by mass from radius in `src/physics.rs` |
| Visual style: game-quality, stunning polish | Partial | Cinematic atmosphere/creature/storm passes and world-first UI are in place; still room for bespoke art-direction polish to hit full "stunning" target |
| Entities are stylized creatures (not simple circles) | Partial | Procedural creature body/head/eye detail and motion are improved; still not full game-art fidelity |
| Attack/eat/death visuals | Implemented | Combat/eat/death particles present |
| Storms as directional wind-line particles | Implemented | Deterministic directional streak fields and gust arcs added in `src/environment.rs` |
| Smooth terrain gradients / rich atmosphere | Partial | Terrain shading/blending and atmosphere overlays improved, but still short of high-end art direction |
| Camera pan/zoom/select/follow + minimap | Implemented | `src/camera.rs`, `src/ui/minimap.rs` |
| HUD controls (play/pause/speed/step) | Implemented | Toolbar supports play/pause/step/1x-10x |
| High-speed runtime controls (adaptive quality + render decimation + sim budget) | Implemented | `src/main.rs`, `src/visual.rs`, `src/ui/settings.rs` |
| Live global stats incl. species count estimate | Implemented | Toolbar computes and displays aggregates |
| Selected entity inspector + neural viz | Implemented | Inspector + neural visualization panels |
| Per-entity sensory overlay toggle | Partial | Ray overlay toggle is global, not per selected entity |
| Phylogenetic tree UI (stretch) | Missing | Not present in UI |
| User tools: spawn food/hazard/entity, smite, draw walls, storm trigger, save/load | Implemented | Main input + settings panel |
| Deterministic seed option | Implemented | Runtime seed option exposed via CLI (`--seed`, snapshot seed path) in `src/main.rs` |
| Performance target: 200+ entities at 60fps on Apple Silicon | Implemented | Reproducible benchmark/report pipeline with pass/fail criteria in `src/main.rs`, `src/reporting.rs` |
| Single-command run with procedural assets | Implemented | `cargo run --release`, no external assets needed |
| Modular codebase + README | Implemented | Modular `src/` layout and README present |

## Priority Missing Items (for your stated goal)

1. Deeper ecological pressure loops and trait tradeoffs (for example stricter size-vs-efficiency coupling and harder famine pressure).
2. UI stretch goals: per-selected-entity sensory overlay controls and phylogenetic lineage view.
3. Final bespoke art-direction polish (palette consistency, richer creature micro-detail, atmospheric composition tuning).

## Suggested Next Build Order

1. Ecology difficulty/tradeoff tuning for richer long-horizon emergence.
2. Stretch UI systems (per-entity sensory overlays and phylogenetic tooling).
3. Final bespoke art-direction sprint to push from cinematic to portfolio-grade "stunning."
