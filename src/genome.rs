use ::rand::Rng;
use macroquad::prelude::*;

use crate::config;

/// Number of neurons in the CTRNN brain.
pub const N: usize = config::BRAIN_NEURONS; // 12

/// Total genome floats for neural params: N*N weights + N biases + N taus.
pub const NEURAL_GENOME_SIZE: usize = N * N + N + N; // 144 + 12 + 12 = 168

/// Full genome including body parameters.
#[derive(Clone, Debug)]
pub struct Genome {
    /// Raw genome values, all normalized to roughly [0, 1].
    /// Layout: [weights: N*N] [biases: N] [taus: N] [body_params: 8]
    pub genes: Vec<f32>,
}

// Body param indices (offsets from NEURAL_GENOME_SIZE)
const BODY_COLOR_R: usize = 0;
const BODY_COLOR_G: usize = 1;
const BODY_COLOR_B: usize = 2;
const BODY_SIZE: usize = 3;
const BODY_MAX_SPEED: usize = 4;
const BODY_SENSOR_RANGE: usize = 5;
const BODY_METABOLIC_RATE: usize = 6;
const BODY_MUTATION_RATE: usize = 7;

pub const BODY_PARAMS_COUNT: usize = 8;
pub const TOTAL_GENOME_SIZE: usize = NEURAL_GENOME_SIZE + BODY_PARAMS_COUNT; // 176

impl Genome {
    pub fn random(rng: &mut impl Rng) -> Self {
        let genes: Vec<f32> = (0..TOTAL_GENOME_SIZE).map(|_| rng.gen_range(0.0..1.0)).collect();
        Self { genes }
    }

    /// Mutate this genome, returning a new child genome.
    pub fn mutate(&self, rng: &mut impl Rng) -> Self {
        let mut child = self.clone();
        let rate = self.mutation_rate();
        let sigma = config::MUTATION_SIGMA;

        for gene in &mut child.genes {
            if rng.gen::<f32>() < rate {
                *gene += rng.gen_range(-sigma..sigma);
                *gene = gene.clamp(0.0, 1.0);
            }
        }

        child
    }

    // --- Weight/Bias/Tau decoding ---

    /// Decode weight W[i][j] from gene. Maps [0,1] -> [-16, 16].
    pub fn weight(&self, i: usize, j: usize) -> f32 {
        (self.genes[i * N + j] - 0.5) * 32.0
    }

    /// Decode bias for neuron i. Maps [0,1] -> [-16, 16].
    pub fn bias(&self, i: usize) -> f32 {
        (self.genes[N * N + i] - 0.5) * 32.0
    }

    /// Decode time constant for neuron i. Maps [0,1] -> [0.5, 5.0].
    pub fn tau(&self, i: usize) -> f32 {
        0.5 + self.genes[N * N + N + i] * 4.5
    }

    // --- Body parameter decoding ---

    fn body_gene(&self, offset: usize) -> f32 {
        self.genes[NEURAL_GENOME_SIZE + offset]
    }

    pub fn body_color(&self) -> Color {
        Color::new(
            0.2 + self.body_gene(BODY_COLOR_R) * 0.8,
            0.2 + self.body_gene(BODY_COLOR_G) * 0.8,
            0.2 + self.body_gene(BODY_COLOR_B) * 0.8,
            1.0,
        )
    }

    /// Body size multiplier [0.6, 1.6].
    pub fn body_size(&self) -> f32 {
        0.6 + self.body_gene(BODY_SIZE) * 1.0
    }

    /// Max speed multiplier [0.5, 1.5].
    pub fn max_speed(&self) -> f32 {
        0.5 + self.body_gene(BODY_MAX_SPEED) * 1.0
    }

    /// Sensor range multiplier [0.5, 1.5].
    pub fn sensor_range(&self) -> f32 {
        0.5 + self.body_gene(BODY_SENSOR_RANGE) * 1.0
    }

    /// Metabolic rate multiplier [0.5, 1.5].
    pub fn metabolic_rate(&self) -> f32 {
        0.5 + self.body_gene(BODY_METABOLIC_RATE) * 1.0
    }

    /// Mutation rate (evolvable meta-parameter) [0.01, 0.15].
    pub fn mutation_rate(&self) -> f32 {
        0.01 + self.body_gene(BODY_MUTATION_RATE) * 0.14
    }
}
