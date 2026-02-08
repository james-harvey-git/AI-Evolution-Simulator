use macroquad::prelude::*;

use crate::entity::EntityArena;
use crate::world::World;

/// Low-resolution pheromone grid for chemical trail signalling.
pub struct PheromoneGrid {
    pub cells: Vec<f32>,
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    inv_cell_size: f32,
}

impl PheromoneGrid {
    pub fn new(world_width: f32, world_height: f32, cell_size: f32) -> Self {
        let width = (world_width / cell_size).ceil() as usize;
        let height = (world_height / cell_size).ceil() as usize;
        Self {
            cells: vec![0.0; width * height],
            width,
            height,
            cell_size,
            inv_cell_size: 1.0 / cell_size,
        }
    }

    /// Deposit pheromone at a world position.
    pub fn deposit(&mut self, pos: Vec2, amount: f32) {
        let cx = ((pos.x * self.inv_cell_size) as usize).min(self.width - 1);
        let cy = ((pos.y * self.inv_cell_size) as usize).min(self.height - 1);
        self.cells[cy * self.width + cx] += amount;
    }

    /// Sample pheromone intensity at a world position.
    pub fn sample(&self, pos: Vec2) -> f32 {
        let cx = ((pos.x * self.inv_cell_size) as usize).min(self.width - 1);
        let cy = ((pos.y * self.inv_cell_size) as usize).min(self.height - 1);
        self.cells[cy * self.width + cx]
    }

    /// Sample the pheromone gradient (direction of increasing concentration).
    pub fn gradient(&self, pos: Vec2) -> Vec2 {
        let cx = (pos.x * self.inv_cell_size) as i32;
        let cy = (pos.y * self.inv_cell_size) as i32;

        let sample = |x: i32, y: i32| -> f32 {
            let x = x.rem_euclid(self.width as i32) as usize;
            let y = y.rem_euclid(self.height as i32) as usize;
            self.cells[y * self.width + x]
        };

        let dx = sample(cx + 1, cy) - sample(cx - 1, cy);
        let dy = sample(cx, cy + 1) - sample(cx, cy - 1);

        vec2(dx, dy) * 0.5
    }

    /// Exponential decay of all pheromones.
    pub fn decay(&mut self, rate: f32, dt: f32) {
        let factor = 1.0 - rate * dt;
        let factor = factor.max(0.0);
        for cell in &mut self.cells {
            *cell *= factor;
        }
    }
}

/// RGB signal that entities broadcast (visible to nearby entities).
#[derive(Clone, Copy, Debug)]
pub struct SignalState {
    pub color: Color,
    pub intensity: f32, // [0, 1]
}

impl Default for SignalState {
    fn default() -> Self {
        Self {
            color: Color::new(0.5, 0.5, 0.5, 0.0),
            intensity: 0.0,
        }
    }
}

/// Update signals and pheromones for all entities.
pub fn update_signals(
    arena: &EntityArena,
    signal_intensities: &[f32], // brain output [0,1] per slot
    signals: &mut Vec<SignalState>,
    pheromone_grid: &mut PheromoneGrid,
    dt: f32,
) {
    // Ensure signals vec is large enough
    if signals.len() < arena.entities.len() {
        signals.resize(arena.entities.len(), SignalState::default());
    }

    for (idx, entity) in arena.entities.iter().enumerate() {
        if let Some(e) = entity {
            let intensity = if idx < signal_intensities.len() {
                signal_intensities[idx]
            } else {
                0.0
            };

            signals[idx] = SignalState {
                color: e.color,
                intensity,
            };

            // Deposit pheromone proportional to movement speed
            let speed = e.velocity.length();
            let deposit_amount = speed * 0.01 * dt;
            if deposit_amount > 0.001 {
                pheromone_grid.deposit(e.pos, deposit_amount);
            }
        } else {
            if idx < signals.len() {
                signals[idx] = SignalState::default();
            }
        }
    }

    // Decay pheromones
    pheromone_grid.decay(0.5, dt); // ~2 second half-life
}

/// Draw signal auras around entities (called from renderer).
pub fn draw_signal_aura(pos: Vec2, radius: f32, signal: &SignalState) {
    if signal.intensity > 0.05 {
        let aura_radius = radius * (2.0 + signal.intensity * 2.0);
        let alpha = signal.intensity * 0.25;
        draw_circle(
            pos.x,
            pos.y,
            aura_radius,
            Color::new(signal.color.r, signal.color.g, signal.color.b, alpha),
        );
    }
}

/// Draw pheromone grid as a semi-transparent heatmap overlay.
pub fn draw_pheromone_overlay(grid: &PheromoneGrid, _world: &World) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let val = grid.cells[y * grid.width + x];
            if val > 0.01 {
                let intensity = val.min(1.0);
                let color = Color::new(0.6, 0.3, 0.8, intensity * 0.15);
                draw_rectangle(
                    x as f32 * grid.cell_size,
                    y as f32 * grid.cell_size,
                    grid.cell_size,
                    grid.cell_size,
                    color,
                );
            }
        }
    }
}
