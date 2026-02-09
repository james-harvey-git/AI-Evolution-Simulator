use crate::config;
use crate::genome::Genome;

/// CTRNN brain storage where each slot can have its own neuron count.
pub struct BrainStorage {
    pub capacity: usize,
    /// Neuron internal states. [slot][neuron]
    pub states: Vec<Vec<f32>>,
    /// Decoded time constants (1/tau). [slot][neuron]
    pub tau_inv: Vec<Vec<f32>>,
    /// Decoded biases. [slot][neuron]
    pub biases: Vec<Vec<f32>>,
    /// Decoded row-major weight matrix. [slot][to * n + from]
    pub weights: Vec<Vec<f32>>,
    /// Output activations: sigmoid(state + bias). [slot][neuron]
    pub outputs: Vec<Vec<f32>>,
    /// Whether this slot is active.
    pub active: Vec<bool>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MotorOutputs {
    pub forward: f32,
    pub turn: f32,
    pub eat: f32,
    pub attack: f32,
    pub share: f32,
    pub pickup: f32,
    pub reproduce: f32,
    pub signal_rgb: [f32; 3],
}

impl BrainStorage {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            states: vec![Vec::new(); capacity],
            tau_inv: vec![Vec::new(); capacity],
            biases: vec![Vec::new(); capacity],
            weights: vec![Vec::new(); capacity],
            outputs: vec![Vec::new(); capacity],
            active: vec![false; capacity],
        }
    }

    /// Initialize a brain slot from a genome.
    pub fn init_from_genome(&mut self, slot: usize, genome: &Genome) {
        self.ensure_capacity(slot + 1);

        let n = genome.total_neurons();
        let mut states = vec![0.0; n];
        let mut tau_inv = vec![1.0; n];
        let mut biases = vec![0.0; n];
        let mut weights = vec![0.0; n * n];
        let outputs = vec![0.0; n];

        // Initialize sensors at 0; other states are also 0 initially.
        for i in 0..n {
            states[i] = 0.0;
            tau_inv[i] = 1.0 / genome.tau(i);
            biases[i] = genome.bias(i);
        }

        for to in 0..n {
            for from in 0..n {
                weights[to * n + from] = genome.weight(to, from);
            }
        }

        self.states[slot] = states;
        self.tau_inv[slot] = tau_inv;
        self.biases[slot] = biases;
        self.weights[slot] = weights;
        self.outputs[slot] = outputs;
        self.active[slot] = true;
    }

    /// Deactivate a brain slot.
    pub fn deactivate(&mut self, slot: usize) {
        if slot < self.active.len() {
            self.active[slot] = false;
        }
    }

    fn ensure_capacity(&mut self, needed: usize) {
        if needed > self.capacity {
            let new_cap = needed.max(self.capacity * 2);
            self.states.resize(new_cap, Vec::new());
            self.tau_inv.resize(new_cap, Vec::new());
            self.biases.resize(new_cap, Vec::new());
            self.weights.resize(new_cap, Vec::new());
            self.outputs.resize(new_cap, Vec::new());
            self.active.resize(new_cap, false);
            self.capacity = new_cap;
        }
    }

    /// Step all active brains one tick using forward Euler integration.
    pub fn step_all(&mut self, sensor_inputs: &[[f32; config::BRAIN_SENSOR_NEURONS]], dt: f32) {
        let sensor_n = config::BRAIN_SENSOR_NEURONS;

        for slot in 0..self.active.len() {
            if !self.active[slot] {
                continue;
            }

            let n = self.states[slot].len();
            if n < sensor_n + config::BRAIN_MOTOR_NEURONS {
                continue;
            }

            let states = &mut self.states[slot];
            let tau_inv = &self.tau_inv[slot];
            let biases = &self.biases[slot];
            let weights = &self.weights[slot];

            // Clamp sensor neurons to input values.
            if slot < sensor_inputs.len() {
                for i in 0..sensor_n {
                    states[i] = sensor_inputs[slot][i];
                }
            }

            // Compute activations for all neurons: sigmoid(state)
            let mut activations = vec![0.0f32; n];
            for i in 0..n {
                activations[i] = sigmoid(states[i]);
            }

            // Forward Euler update for non-sensor neurons.
            for i in sensor_n..n {
                let mut input_sum = biases[i];
                let row = i * n;
                for j in 0..n {
                    input_sum += weights[row + j] * activations[j];
                }
                let dydt = (-states[i] + input_sum) * tau_inv[i];
                states[i] += dydt * dt;
                states[i] = states[i].clamp(-20.0, 20.0);
            }

            // Compute final output activations.
            let outputs = &mut self.outputs[slot];
            if outputs.len() != n {
                outputs.resize(n, 0.0);
            }
            for i in 0..n {
                outputs[i] = sigmoid(states[i]);
            }
        }
    }

    /// Get motor outputs for a slot.
    /// Motor neurons are the final 10 channels in the neuron ordering.
    pub fn motor_outputs(&self, slot: usize) -> MotorOutputs {
        let o = &self.outputs[slot];
        if o.len() < config::BRAIN_MOTOR_NEURONS {
            return MotorOutputs::default();
        }

        let motor_start = o.len() - config::BRAIN_MOTOR_NEURONS;

        MotorOutputs {
            forward: o[motor_start],
            turn: o[motor_start + 1] * 2.0 - 1.0,
            eat: o[motor_start + 2],
            attack: o[motor_start + 3],
            share: o[motor_start + 4],
            pickup: o[motor_start + 5],
            reproduce: o[motor_start + 6],
            signal_rgb: [o[motor_start + 7], o[motor_start + 8], o[motor_start + 9]],
        }
    }

    pub fn neuron_count(&self, slot: usize) -> Option<usize> {
        self.states.get(slot).map(Vec::len).filter(|n| *n > 0)
    }

    pub fn interneuron_count(&self, slot: usize) -> Option<usize> {
        self.neuron_count(slot).and_then(|n| {
            let baseline = config::BRAIN_SENSOR_NEURONS + config::BRAIN_MOTOR_NEURONS;
            n.checked_sub(baseline)
        })
    }

    pub fn slot_states(&self, slot: usize) -> Option<&[f32]> {
        self.states.get(slot).map(Vec::as_slice)
    }

    pub fn slot_outputs(&self, slot: usize) -> Option<&[f32]> {
        self.outputs.get(slot).map(Vec::as_slice)
    }

    pub fn slot_weights(&self, slot: usize) -> Option<&[f32]> {
        self.weights.get(slot).map(Vec::as_slice)
    }
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn init_from_genome_respects_variable_neuron_count() {
        let mut rng = ChaCha8Rng::seed_from_u64(41);
        let g_small = Genome::random_with_inter_neurons(&mut rng, config::BRAIN_MIN_INTERNEURONS);
        let g_large = Genome::random_with_inter_neurons(&mut rng, config::BRAIN_MAX_INTERNEURONS);

        let mut brains = BrainStorage::new(2);
        brains.init_from_genome(0, &g_small);
        brains.init_from_genome(1, &g_large);

        assert_eq!(brains.neuron_count(0), Some(g_small.total_neurons()));
        assert_eq!(brains.neuron_count(1), Some(g_large.total_neurons()));
        assert_eq!(
            brains.interneuron_count(0),
            Some(config::BRAIN_MIN_INTERNEURONS)
        );
        assert_eq!(
            brains.interneuron_count(1),
            Some(config::BRAIN_MAX_INTERNEURONS)
        );
    }

    #[test]
    fn motor_output_mapping_uses_last_ten_channels() {
        let mut brains = BrainStorage::new(1);
        brains.active[0] = true;
        brains.outputs[0] =
            vec![0.0; config::BRAIN_SENSOR_NEURONS + 5 + config::BRAIN_MOTOR_NEURONS];

        let start = brains.outputs[0].len() - config::BRAIN_MOTOR_NEURONS;
        brains.outputs[0][start] = 0.8;
        brains.outputs[0][start + 1] = 0.25;
        brains.outputs[0][start + 2] = 0.6;
        brains.outputs[0][start + 3] = 0.7;
        brains.outputs[0][start + 4] = 0.2;
        brains.outputs[0][start + 5] = 0.9;
        brains.outputs[0][start + 6] = 0.4;
        brains.outputs[0][start + 7] = 0.1;
        brains.outputs[0][start + 8] = 0.3;
        brains.outputs[0][start + 9] = 0.5;

        let m = brains.motor_outputs(0);
        assert!((m.forward - 0.8).abs() < 1e-6);
        assert!((m.turn - (-0.5)).abs() < 1e-6);
        assert!((m.eat - 0.6).abs() < 1e-6);
        assert!((m.attack - 0.7).abs() < 1e-6);
        assert!((m.share - 0.2).abs() < 1e-6);
        assert!((m.pickup - 0.9).abs() < 1e-6);
        assert!((m.reproduce - 0.4).abs() < 1e-6);
        assert_eq!(m.signal_rgb, [0.1, 0.3, 0.5]);
    }

    #[test]
    fn step_all_clamps_sensors_and_produces_finite_outputs() {
        let mut rng = ChaCha8Rng::seed_from_u64(55);
        let genome = Genome::random_with_inter_neurons(&mut rng, 8);
        let mut brains = BrainStorage::new(1);
        brains.init_from_genome(0, &genome);

        let mut sensors = vec![[0.0f32; config::BRAIN_SENSOR_NEURONS]; 1];
        for i in 0..config::BRAIN_SENSOR_NEURONS {
            sensors[0][i] = i as f32 / config::BRAIN_SENSOR_NEURONS as f32;
        }

        brains.step_all(&sensors, 1.0 / 60.0);

        let states = brains.slot_states(0).unwrap();
        for (i, v) in states.iter().take(config::BRAIN_SENSOR_NEURONS).enumerate() {
            assert!((*v - sensors[0][i]).abs() < 1e-6);
        }

        let outputs = brains.slot_outputs(0).unwrap();
        assert!(outputs.iter().all(|v| v.is_finite()));
    }
}
