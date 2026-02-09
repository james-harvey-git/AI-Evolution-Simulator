use ::rand::Rng;
use macroquad::prelude::*;

use crate::config;

// Body param indices (offsets from neural genome block)
const BODY_COLOR_R: usize = 0;
const BODY_COLOR_G: usize = 1;
const BODY_COLOR_B: usize = 2;
const BODY_SIZE: usize = 3;
const BODY_MAX_SPEED: usize = 4;
const BODY_SENSOR_RANGE: usize = 5;
const BODY_METABOLIC_RATE: usize = 6;
const BODY_MUTATION_RATE: usize = 7;
const BODY_MUTATION_SIGMA: usize = 8;

pub const BODY_PARAMS_COUNT: usize = 9;

/// Default neural genome size, kept for legacy compatibility only.
pub const NEURAL_GENOME_SIZE: usize = config::BRAIN_NEURONS_DEFAULT * config::BRAIN_NEURONS_DEFAULT
    + config::BRAIN_NEURONS_DEFAULT * 2;

/// Default full genome size, kept for legacy compatibility only.
pub const TOTAL_GENOME_SIZE: usize = NEURAL_GENOME_SIZE + BODY_PARAMS_COUNT;

/// Variable-topology genome for a CTRNN entity brain.
#[derive(Clone, Debug)]
pub struct Genome {
    inter_neurons: u8,
    /// Raw genes in [0,1].
    /// Layout: [weights: n*n] [biases: n] [taus: n] [body_params: 9]
    /// where n = sensor + inter + motor.
    pub genes: Vec<f32>,
}

impl Genome {
    pub fn random(rng: &mut impl Rng) -> Self {
        Self::random_with_inter_neurons(rng, config::BRAIN_INTERNEURONS_DEFAULT)
    }

    pub fn random_with_inter_neurons(rng: &mut impl Rng, inter_neurons: usize) -> Self {
        let inter_neurons = Self::clamp_interneuron_count(inter_neurons);
        let len = Self::total_gene_len_for_inter(inter_neurons);
        let genes: Vec<f32> = (0..len).map(|_| rng.gen_range(0.0..1.0)).collect();
        Self {
            inter_neurons: inter_neurons as u8,
            genes,
        }
    }

    /// Construct from persisted genes, normalizing to the expected size for `inter_neurons`.
    pub fn from_raw(inter_neurons: usize, mut genes: Vec<f32>) -> Self {
        let inter_neurons = Self::clamp_interneuron_count(inter_neurons);
        let expected = Self::total_gene_len_for_inter(inter_neurons);
        if genes.len() < expected {
            genes.resize(expected, 0.5);
        } else if genes.len() > expected {
            genes.truncate(expected);
        }

        Self {
            inter_neurons: inter_neurons as u8,
            genes,
        }
    }

    pub fn clamp_interneuron_count(inter_neurons: usize) -> usize {
        inter_neurons.clamp(
            config::BRAIN_MIN_INTERNEURONS,
            config::BRAIN_MAX_INTERNEURONS,
        )
    }

    pub fn inter_neurons(&self) -> usize {
        self.inter_neurons as usize
    }

    pub fn total_neurons(&self) -> usize {
        config::BRAIN_SENSOR_NEURONS + self.inter_neurons() + config::BRAIN_MOTOR_NEURONS
    }

    pub fn neural_gene_len(&self) -> usize {
        let n = self.total_neurons();
        n * n + n + n
    }

    pub fn total_gene_len(&self) -> usize {
        self.neural_gene_len() + BODY_PARAMS_COUNT
    }

    pub fn total_gene_len_for_inter(inter_neurons: usize) -> usize {
        let n = config::BRAIN_SENSOR_NEURONS
            + Self::clamp_interneuron_count(inter_neurons)
            + config::BRAIN_MOTOR_NEURONS;
        n * n + n + n + BODY_PARAMS_COUNT
    }

    /// Mutate this genome, returning a new child genome.
    pub fn mutate(&self, rng: &mut impl Rng) -> Self {
        let mut child = self.clone();
        let rate = child.mutation_rate();
        let sigma = child.mutation_sigma();

        for gene in &mut child.genes {
            if rng.gen::<f32>() < rate {
                *gene += rng.gen_range(-sigma..sigma);
                *gene = gene.clamp(0.0, 1.0);
            }
        }

        if rng.gen::<f32>() < config::STRUCTURAL_ADD_PROB {
            if child.inter_neurons() < config::BRAIN_MAX_INTERNEURONS {
                child.add_interneuron_at_end_of_inter_block(rng);
            }
        } else if rng.gen::<f32>() < config::STRUCTURAL_REMOVE_PROB {
            if child.inter_neurons() > config::BRAIN_MIN_INTERNEURONS {
                child.remove_random_interneuron(rng);
            }
        }

        child
    }

    /// Add one interneuron after the current interneuron block.
    pub(crate) fn add_interneuron_at_end_of_inter_block(&mut self, rng: &mut impl Rng) {
        let old_inter = self.inter_neurons();
        if old_inter >= config::BRAIN_MAX_INTERNEURONS {
            return;
        }

        let old_n = self.total_neurons();
        let insert_idx = config::BRAIN_SENSOR_NEURONS + old_inter;
        let new_n = old_n + 1;

        let old_weights_end = old_n * old_n;
        let old_biases_end = old_weights_end + old_n;
        let old_taus_end = old_biases_end + old_n;

        let old_weights = &self.genes[0..old_weights_end];
        let old_biases = &self.genes[old_weights_end..old_biases_end];
        let old_taus = &self.genes[old_biases_end..old_taus_end];
        let body = self.body_genes_copy();

        let new_neural_len = new_n * new_n + new_n + new_n;
        let mut new_genes = vec![0.0; new_neural_len + BODY_PARAMS_COUNT];

        // Weights
        for to_new in 0..new_n {
            for from_new in 0..new_n {
                let v = if to_new == insert_idx || from_new == insert_idx {
                    rng.gen_range(0.0..1.0)
                } else {
                    let to_old = if to_new > insert_idx {
                        to_new - 1
                    } else {
                        to_new
                    };
                    let from_old = if from_new > insert_idx {
                        from_new - 1
                    } else {
                        from_new
                    };
                    old_weights[to_old * old_n + from_old]
                };
                new_genes[to_new * new_n + from_new] = v;
            }
        }

        // Biases
        let new_bias_base = new_n * new_n;
        for i_new in 0..new_n {
            let v = if i_new == insert_idx {
                rng.gen_range(0.0..1.0)
            } else {
                let i_old = if i_new > insert_idx { i_new - 1 } else { i_new };
                old_biases[i_old]
            };
            new_genes[new_bias_base + i_new] = v;
        }

        // Taus
        let new_tau_base = new_bias_base + new_n;
        for i_new in 0..new_n {
            let v = if i_new == insert_idx {
                rng.gen_range(0.0..1.0)
            } else {
                let i_old = if i_new > insert_idx { i_new - 1 } else { i_new };
                old_taus[i_old]
            };
            new_genes[new_tau_base + i_new] = v;
        }

        // Body params
        let new_body_base = new_tau_base + new_n;
        for (i, gene) in body.iter().enumerate() {
            new_genes[new_body_base + i] = *gene;
        }

        self.genes = new_genes;
        self.inter_neurons = (old_inter + 1) as u8;
    }

    /// Remove one random interneuron.
    pub(crate) fn remove_random_interneuron(&mut self, rng: &mut impl Rng) {
        let old_inter = self.inter_neurons();
        if old_inter <= config::BRAIN_MIN_INTERNEURONS {
            return;
        }

        let old_n = self.total_neurons();
        let remove_idx = config::BRAIN_SENSOR_NEURONS + rng.gen_range(0..old_inter);
        let new_n = old_n - 1;

        let old_weights_end = old_n * old_n;
        let old_biases_end = old_weights_end + old_n;
        let old_taus_end = old_biases_end + old_n;

        let old_weights = &self.genes[0..old_weights_end];
        let old_biases = &self.genes[old_weights_end..old_biases_end];
        let old_taus = &self.genes[old_biases_end..old_taus_end];
        let body = self.body_genes_copy();

        let new_neural_len = new_n * new_n + new_n + new_n;
        let mut new_genes = vec![0.0; new_neural_len + BODY_PARAMS_COUNT];

        // Weights
        for to_new in 0..new_n {
            for from_new in 0..new_n {
                let to_old = if to_new >= remove_idx {
                    to_new + 1
                } else {
                    to_new
                };
                let from_old = if from_new >= remove_idx {
                    from_new + 1
                } else {
                    from_new
                };
                new_genes[to_new * new_n + from_new] = old_weights[to_old * old_n + from_old];
            }
        }

        // Biases
        let new_bias_base = new_n * new_n;
        for i_new in 0..new_n {
            let i_old = if i_new >= remove_idx {
                i_new + 1
            } else {
                i_new
            };
            new_genes[new_bias_base + i_new] = old_biases[i_old];
        }

        // Taus
        let new_tau_base = new_bias_base + new_n;
        for i_new in 0..new_n {
            let i_old = if i_new >= remove_idx {
                i_new + 1
            } else {
                i_new
            };
            new_genes[new_tau_base + i_new] = old_taus[i_old];
        }

        // Body params
        let new_body_base = new_tau_base + new_n;
        for (i, gene) in body.iter().enumerate() {
            new_genes[new_body_base + i] = *gene;
        }

        self.genes = new_genes;
        self.inter_neurons = (old_inter - 1) as u8;
    }

    // --- Weight/Bias/Tau decoding ---

    /// Decode weight W[i][j] from gene. Maps [0,1] -> [-16, 16].
    pub fn weight(&self, i: usize, j: usize) -> f32 {
        let n = self.total_neurons();
        (self.genes[i * n + j] - 0.5) * 32.0
    }

    /// Decode bias for neuron i. Maps [0,1] -> [-16, 16].
    pub fn bias(&self, i: usize) -> f32 {
        let n = self.total_neurons();
        (self.genes[n * n + i] - 0.5) * 32.0
    }

    /// Decode time constant for neuron i. Maps [0,1] -> [0.5, 5.0].
    pub fn tau(&self, i: usize) -> f32 {
        let n = self.total_neurons();
        0.5 + self.genes[n * n + n + i] * 4.5
    }

    // --- Body parameter decoding ---

    fn body_gene(&self, offset: usize) -> f32 {
        self.genes[self.neural_gene_len() + offset]
    }

    fn body_genes_copy(&self) -> Vec<f32> {
        let mut body = vec![0.5; BODY_PARAMS_COUNT];
        let start = self.neural_gene_len();
        let available = self
            .genes
            .len()
            .saturating_sub(start)
            .min(BODY_PARAMS_COUNT);
        for (i, slot) in body.iter_mut().take(available).enumerate() {
            *slot = self.genes[start + i];
        }
        body
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

    /// Mutation sigma (evolvable meta-parameter) [0.02, 0.30].
    pub fn mutation_sigma(&self) -> f32 {
        0.02 + self.body_gene(BODY_MUTATION_SIGMA) * 0.28
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn random_genome_has_expected_length_and_range() {
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let genome = Genome::random(&mut rng);

        assert_eq!(genome.inter_neurons(), config::BRAIN_INTERNEURONS_DEFAULT);
        assert_eq!(genome.genes.len(), genome.total_gene_len());
        assert!(genome.genes.iter().all(|g| *g >= 0.0 && *g <= 1.0));
    }

    #[test]
    fn mutation_keeps_gene_values_clamped_and_bounds_topology() {
        let mut rng = ChaCha8Rng::seed_from_u64(11);
        let genome = Genome::random(&mut rng);
        let child = genome.mutate(&mut rng);

        assert!(child.inter_neurons() >= config::BRAIN_MIN_INTERNEURONS);
        assert!(child.inter_neurons() <= config::BRAIN_MAX_INTERNEURONS);
        assert_eq!(child.genes.len(), child.total_gene_len());
        assert!(child.genes.iter().all(|g| *g >= 0.0 && *g <= 1.0));
    }

    #[test]
    fn structural_helpers_preserve_length_invariants() {
        let mut rng = ChaCha8Rng::seed_from_u64(31);
        let mut genome =
            Genome::random_with_inter_neurons(&mut rng, config::BRAIN_INTERNEURONS_DEFAULT);

        let before = genome.inter_neurons();
        genome.add_interneuron_at_end_of_inter_block(&mut rng);
        assert_eq!(genome.inter_neurons(), before + 1);
        assert_eq!(genome.genes.len(), genome.total_gene_len());

        genome.remove_random_interneuron(&mut rng);
        assert_eq!(genome.inter_neurons(), before);
        assert_eq!(genome.genes.len(), genome.total_gene_len());
    }

    #[test]
    fn decode_ranges_match_contract() {
        let n = config::BRAIN_SENSOR_NEURONS
            + config::BRAIN_INTERNEURONS_DEFAULT
            + config::BRAIN_MOTOR_NEURONS;
        let zero = Genome::from_raw(
            config::BRAIN_INTERNEURONS_DEFAULT,
            vec![0.0; n * n + n + n + BODY_PARAMS_COUNT],
        );
        let one = Genome::from_raw(
            config::BRAIN_INTERNEURONS_DEFAULT,
            vec![1.0; n * n + n + n + BODY_PARAMS_COUNT],
        );

        assert!((zero.weight(0, 0) + 16.0).abs() < f32::EPSILON);
        assert!((one.weight(0, 0) - 16.0).abs() < f32::EPSILON);
        assert!((zero.bias(0) + 16.0).abs() < f32::EPSILON);
        assert!((one.bias(0) - 16.0).abs() < f32::EPSILON);
        assert!((zero.tau(0) - 0.5).abs() < f32::EPSILON);
        assert!((one.tau(0) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn from_raw_normalizes_gene_length_for_requested_topology() {
        let inter = config::BRAIN_MIN_INTERNEURONS;
        let expected = Genome::total_gene_len_for_inter(inter);
        let g = Genome::from_raw(inter, vec![0.25; expected - 5]);
        assert_eq!(g.inter_neurons(), inter);
        assert_eq!(g.genes.len(), expected);
        assert!((g.genes[expected - 1] - 0.5).abs() < 1e-6);
    }
}
