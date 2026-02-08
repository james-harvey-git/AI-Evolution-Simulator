use macroquad::prelude::*;

const MAX_PARTICLES: usize = 500;

#[derive(Clone, Copy)]
struct Particle {
    pos: Vec2,
    velocity: Vec2,
    color: Color,
    life: f32,
    max_life: f32,
    size: f32,
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
        }
    }

    /// Burst effect for entity birth (white/cyan sparkles).
    pub fn emit_birth(&mut self, pos: Vec2) {
        self.emit_burst(pos, 12, Color::new(0.8, 0.95, 1.0, 1.0), 60.0, 0.6);
    }

    /// Burst effect for entity death (red fade).
    pub fn emit_death(&mut self, pos: Vec2) {
        self.emit_burst(pos, 16, Color::new(1.0, 0.2, 0.1, 1.0), 40.0, 0.8);
    }

    /// Small burst for eating food (green).
    pub fn emit_eat(&mut self, pos: Vec2) {
        self.emit_burst(pos, 6, Color::new(0.2, 0.9, 0.3, 0.9), 30.0, 0.4);
    }

    /// Burst for combat hit (yellow/orange).
    pub fn emit_combat(&mut self, pos: Vec2) {
        self.emit_burst(pos, 10, Color::new(1.0, 0.7, 0.1, 1.0), 50.0, 0.5);
    }

    fn emit_burst(&mut self, pos: Vec2, count: usize, color: Color, speed: f32, lifetime: f32) {
        for i in 0..count {
            if self.particles.len() >= MAX_PARTICLES {
                // Remove oldest particle
                self.particles.remove(0);
            }

            let angle = (i as f32 / count as f32) * std::f32::consts::TAU
                + rand::gen_range(-0.3, 0.3);
            let spd = speed * rand::gen_range(0.4, 1.0);
            let vel = Vec2::from_angle(angle) * spd;

            self.particles.push(Particle {
                pos,
                velocity: vel,
                color,
                life: lifetime * rand::gen_range(0.7, 1.0),
                max_life: lifetime,
                size: rand::gen_range(1.5, 3.5),
            });
        }
    }

    /// Update all particles, removing expired ones.
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.pos += p.velocity * dt;
            p.velocity *= 1.0 - 2.0 * dt; // drag
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Draw all particles.
    pub fn draw(&self) {
        for p in &self.particles {
            let t = (p.life / p.max_life).clamp(0.0, 1.0);
            let alpha = t * p.color.a;
            let size = p.size * (0.3 + 0.7 * t);
            let color = Color::new(p.color.r, p.color.g, p.color.b, alpha);
            draw_circle(p.pos.x, p.pos.y, size, color);
        }
    }

    pub fn count(&self) -> usize {
        self.particles.len()
    }
}
