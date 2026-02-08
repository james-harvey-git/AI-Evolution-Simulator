use crate::config;
use crate::genome::{Genome, N};

/// CTRNN brain storage in Structure-of-Arrays layout for cache performance.
/// All brains are stored contiguously, indexed by entity slot index.
pub struct BrainStorage {
    pub capacity: usize,
    /// Neuron internal states (membrane potential). [slot][neuron]
    pub states: Vec<[f32; N]>,
    /// Decoded time constants (1/tau for faster computation). [slot][neuron]
    pub tau_inv: Vec<[f32; N]>,
    /// Decoded biases. [slot][neuron]
    pub biases: Vec<[f32; N]>,
    /// Decoded weight matrix W[i][j]. [slot][to][from]
    pub weights: Vec<[[f32; N]; N]>,
    /// Output activations: sigmoid(state + bias). [slot][neuron]
    pub outputs: Vec<[f32; N]>,
    /// Whether this slot is active.
    pub active: Vec<bool>,
}

impl BrainStorage {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            states: vec![[0.0; N]; capacity],
            tau_inv: vec![[1.0; N]; capacity],
            biases: vec![[0.0; N]; capacity],
            weights: vec![[[0.0; N]; N]; capacity],
            outputs: vec![[0.0; N]; capacity],
            active: vec![false; capacity],
        }
    }

    /// Initialize a brain slot from a genome.
    pub fn init_from_genome(&mut self, slot: usize, genome: &Genome) {
        self.ensure_capacity(slot + 1);

        self.states[slot] = [0.0; N];
        for i in 0..N {
            let tau = genome.tau(i);
            self.tau_inv[slot][i] = 1.0 / tau;
            self.biases[slot][i] = genome.bias(i);
        }
        for i in 0..N {
            for j in 0..N {
                self.weights[slot][i][j] = genome.weight(i, j);
            }
        }
        self.outputs[slot] = [0.0; N];
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
            self.states.resize(new_cap, [0.0; N]);
            self.tau_inv.resize(new_cap, [1.0; N]);
            self.biases.resize(new_cap, [0.0; N]);
            self.weights.resize(new_cap, [[0.0; N]; N]);
            self.outputs.resize(new_cap, [0.0; N]);
            self.active.resize(new_cap, false);
            self.capacity = new_cap;
        }
    }

    /// Step all active brains one tick using forward Euler integration.
    ///
    /// Neuron layout:
    ///   0..SENSOR_N: sensor input neurons (states are clamped to input values)
    ///   SENSOR_N..SENSOR_N+INTER_N: interneurons (recurrent dynamics)
    ///   SENSOR_N+INTER_N..N: motor output neurons (read after step)
    ///
    /// sensor_inputs[slot] provides values for sensor neurons.
    pub fn step_all(&mut self, sensor_inputs: &[[f32; config::BRAIN_SENSOR_NEURONS]], dt: f32) {
        let sensor_n = config::BRAIN_SENSOR_NEURONS;

        for slot in 0..self.active.len() {
            if !self.active[slot] {
                continue;
            }

            let states = &mut self.states[slot];
            let tau_inv = &self.tau_inv[slot];
            let biases = &self.biases[slot];
            let weights = &self.weights[slot];

            // Clamp sensor neurons to input values
            if slot < sensor_inputs.len() {
                for i in 0..sensor_n {
                    states[i] = sensor_inputs[slot][i];
                }
            }

            // Compute activations for all neurons: sigmoid(state)
            let mut activations = [0.0f32; N];
            for i in 0..N {
                activations[i] = sigmoid(states[i]);
            }

            // Forward Euler update for non-sensor neurons
            // dy_i/dt = (-y_i + bias_i + sum_j(w_ij * activation_j)) * (1/tau_i)
            for i in sensor_n..N {
                let mut input_sum = biases[i];
                for j in 0..N {
                    input_sum += weights[i][j] * activations[j];
                }
                let dydt = (-states[i] + input_sum) * tau_inv[i];
                states[i] += dydt * dt;

                // Clamp to prevent state explosion
                states[i] = states[i].clamp(-20.0, 20.0);
            }

            // Compute final output activations
            for i in 0..N {
                self.outputs[slot][i] = sigmoid(states[i]);
            }
        }
    }

    /// Get motor outputs for a slot: (forward_drive, turn, attack_intent, signal_intensity).
    /// All values in [0, 1]. Turn is remapped to [-1, 1].
    pub fn motor_outputs(&self, slot: usize) -> (f32, f32, f32, f32) {
        let o = &self.outputs[slot];
        let motor_start = config::BRAIN_SENSOR_NEURONS + config::BRAIN_INTERNEURONS;
        (
            o[motor_start],             // forward drive [0,1]
            o[motor_start + 1] * 2.0 - 1.0, // turn [-1,1]
            o[motor_start + 2],         // attack intent [0,1]
            o[motor_start + 3],         // signal intensity [0,1]
        )
    }
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}
