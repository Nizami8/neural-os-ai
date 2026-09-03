//! Multi-layer perceptron with online momentum SGD.
//!
//! Weights and activations use Q16.16 fixed-point (`fixed::Fixed`) so the
//! scheduling hot path does not depend on soft-float. Public APIs still speak
//! `f32` for host tests and shell display.

use crate::fixed::{self, Fixed, HALF, ONE, ZERO};

const HIDDEN_SIZE: usize = 8;
const INPUT_SIZE: usize = 7;

pub struct NeuralScheduler {
    pub hidden_weights: [[Fixed; INPUT_SIZE]; HIDDEN_SIZE],
    pub hidden_bias: [Fixed; HIDDEN_SIZE],
    pub output_weights: [Fixed; HIDDEN_SIZE],
    pub output_bias: Fixed,
    pub hidden_velocity: [[Fixed; INPUT_SIZE]; HIDDEN_SIZE],
    pub output_velocity: [Fixed; HIDDEN_SIZE],
    pub learning_rate: Fixed,
    pub momentum: Fixed,
}

#[derive(Clone, Copy)]
pub struct TaskMetrics {
    pub task_id: usize,
    pub execution_time: u32,
    pub wait_time: u32,
    pub memory_used: usize,
    pub ticks_since_run: u32,
    pub io_wait_count: u32,
    pub context_switches: u32,
    pub priority_boost: i32,
    pub ema_exec_time: f32,
    pub priority: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TaskClass {
    RealTime,
    Interactive,
    Batch,
}

impl TaskMetrics {
    pub fn new(task_id: usize) -> Self {
        TaskMetrics {
            task_id,
            execution_time: 0,
            wait_time: 0,
            memory_used: 0,
            ticks_since_run: 0,
            io_wait_count: 0,
            context_switches: 0,
            priority_boost: 0,
            ema_exec_time: 0.0,
            priority: 0.0,
        }
    }

    pub fn update_ema(&mut self, new_exec_time: u32) {
        let alpha = 0.3;
        self.ema_exec_time =
            alpha * (new_exec_time as f32) + (1.0 - alpha) * self.ema_exec_time;
    }
}

impl NeuralScheduler {
    pub fn new() -> Self {
        let mut scheduler = NeuralScheduler {
            hidden_weights: [[HALF; INPUT_SIZE]; HIDDEN_SIZE],
            hidden_bias: [Fixed::from_f32(0.1); HIDDEN_SIZE],
            output_weights: [Fixed::from_f32(0.3); HIDDEN_SIZE],
            output_bias: ZERO,
            hidden_velocity: [[ZERO; INPUT_SIZE]; HIDDEN_SIZE],
            output_velocity: [ZERO; HIDDEN_SIZE],
            learning_rate: Fixed::from_f32(0.01),
            momentum: Fixed::from_f32(0.9),
        };

        // LCG init breaks symmetry between neurons.
        let mut seed: u32 = 0x2545_f491;
        let mut rand = || -> Fixed {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let unit = ((seed >> 9) as f32 / 8_388_608.0) * 2.0 - 1.0;
            Fixed::from_f32(unit)
        };
        for i in 0..HIDDEN_SIZE {
            for j in 0..INPUT_SIZE {
                scheduler.hidden_weights[i][j] = HALF + Fixed::from_f32(0.15) * rand();
            }
            scheduler.output_weights[i] = Fixed::from_f32(0.3) + Fixed::from_f32(0.10) * rand();
        }

        scheduler
    }

    fn normalize_inputs(&self, metrics: &TaskMetrics) -> [Fixed; INPUT_SIZE] {
        [
            Fixed::from_f32((metrics.execution_time as f32) / 1000.0),
            Fixed::from_f32((metrics.wait_time as f32) / 1000.0),
            Fixed::from_f32((metrics.memory_used as f32) / 100.0),
            Fixed::from_f32((metrics.ticks_since_run as f32) / 100.0),
            Fixed::from_f32((metrics.io_wait_count as f32) / 100.0),
            Fixed::from_f32((metrics.context_switches as f32) / 50.0),
            Fixed::from_f32((metrics.priority_boost as f32) / 10.0),
        ]
    }

    #[inline]
    fn relu(x: Fixed) -> Fixed {
        if x.0 > 0 {
            x
        } else {
            x * Fixed::from_f32(0.01)
        }
    }

    #[inline]
    fn relu_derivative(x: Fixed) -> Fixed {
        if x.0 > 0 {
            ONE
        } else {
            Fixed::from_f32(0.01)
        }
    }

    fn forward_hidden(&self, inputs: &[Fixed; INPUT_SIZE]) -> [Fixed; HIDDEN_SIZE] {
        let mut hidden = [ZERO; HIDDEN_SIZE];
        for i in 0..HIDDEN_SIZE {
            let mut z = self.hidden_bias[i];
            for j in 0..INPUT_SIZE {
                z += self.hidden_weights[i][j] * inputs[j];
            }
            hidden[i] = Self::relu(z);
        }
        hidden
    }

    fn forward_output(&self, hidden: &[Fixed; HIDDEN_SIZE]) -> Fixed {
        let mut z = self.output_bias;
        for i in 0..HIDDEN_SIZE {
            z += self.output_weights[i] * hidden[i];
        }
        fixed::sigmoid(z)
    }

    pub fn predict_priority(&self, metrics: &TaskMetrics) -> f32 {
        let inputs = self.normalize_inputs(metrics);
        let hidden = self.forward_hidden(&inputs);
        self.forward_output(&hidden).to_f32()
    }

    pub fn learn(&mut self, metrics: &TaskMetrics, target: f32) {
        let inputs = self.normalize_inputs(metrics);
        let hidden = self.forward_hidden(&inputs);
        let output = self.forward_output(&hidden);
        let target_f = Fixed::from_f32(target);
        let output_error = target_f - output;

        if output_error.abs().to_f32() < 0.001 {
            return;
        }

        let output_delta = output_error * fixed::sigmoid_derivative_from_output(output);

        for i in 0..HIDDEN_SIZE {
            let grad = output_delta * hidden[i];
            self.output_velocity[i] =
                self.momentum * self.output_velocity[i] + self.learning_rate * grad;
            self.output_weights[i] += self.output_velocity[i];
        }
        self.output_bias += self.learning_rate * output_delta;

        for i in 0..HIDDEN_SIZE {
            let hidden_delta =
                output_delta * self.output_weights[i] * Self::relu_derivative(hidden[i]);
            for j in 0..INPUT_SIZE {
                let grad = hidden_delta * inputs[j];
                self.hidden_velocity[i][j] =
                    self.momentum * self.hidden_velocity[i][j] + self.learning_rate * grad;
                self.hidden_weights[i][j] += self.hidden_velocity[i][j];
            }
            self.hidden_bias[i] += self.learning_rate * hidden_delta;
        }
    }

    pub fn get_output_weights(&self) -> [f32; HIDDEN_SIZE] {
        let mut out = [0.0; HIDDEN_SIZE];
        for i in 0..HIDDEN_SIZE {
            out[i] = self.output_weights[i].to_f32();
        }
        out
    }

    /// Export flattened hidden + output weights for persistent storage.
    pub fn export_weights(&self) -> ([[f32; INPUT_SIZE]; HIDDEN_SIZE], [f32; HIDDEN_SIZE]) {
        let mut hidden = [[0.0f32; INPUT_SIZE]; HIDDEN_SIZE];
        let mut output = [0.0f32; HIDDEN_SIZE];
        for i in 0..HIDDEN_SIZE {
            for j in 0..INPUT_SIZE {
                hidden[i][j] = self.hidden_weights[i][j].to_f32();
            }
            output[i] = self.output_weights[i].to_f32();
        }
        (hidden, output)
    }

    /// Restore weights previously exported via `export_weights`.
    pub fn import_weights(
        &mut self,
        hidden: &[[f32; INPUT_SIZE]; HIDDEN_SIZE],
        output: &[f32; HIDDEN_SIZE],
    ) {
        for i in 0..HIDDEN_SIZE {
            for j in 0..INPUT_SIZE {
                self.hidden_weights[i][j] = Fixed::from_f32(hidden[i][j]);
            }
            self.output_weights[i] = Fixed::from_f32(output[i]);
            self.hidden_velocity[i] = [ZERO; INPUT_SIZE];
            self.output_velocity[i] = ZERO;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Absolute value that works in both `no_std` and host-test builds.
#[inline]
pub fn fabs(x: f32) -> f32 {
    if x.is_sign_negative() {
        -x
    } else {
        x
    }
}

/// Host/display helper: fixed-point sigmoid exposed as f32.
#[inline]
pub fn sigmoid(x: f32) -> f32 {
    fixed::sigmoid(Fixed::from_f32(x)).to_f32()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoid_is_bounded() {
        assert!(fabs(sigmoid(0.0) - 0.5) < 0.02);
        assert!(sigmoid(20.0) > 0.9);
        assert!(sigmoid(-20.0) < 0.1);
    }

    #[test]
    fn priority_stays_in_unit_interval() {
        let net = NeuralScheduler::new();
        let mut m = TaskMetrics::new(1);
        m.execution_time = 900;
        m.wait_time = 400;
        let p = net.predict_priority(&m);
        assert!((0.0..=1.0).contains(&p), "priority out of range: {}", p);
    }

    #[test]
    fn weight_init_breaks_symmetry() {
        let net = NeuralScheduler::new();
        let w = net.get_output_weights();
        assert!(
            w.iter().any(|&x| fabs(x - w[0]) > 1e-4),
            "output weights are degenerate/symmetric: {:?}",
            w
        );
    }

    #[test]
    fn training_reduces_error() {
        let mut net = NeuralScheduler::new();
        let mut m = TaskMetrics::new(1);
        m.execution_time = 500;
        m.wait_time = 300;
        m.io_wait_count = 4;
        let target = 0.9;
        let before = fabs(target - net.predict_priority(&m));
        for _ in 0..2000 {
            net.learn(&m, target);
        }
        let after = fabs(target - net.predict_priority(&m));
        assert!(
            after < before,
            "SGD failed to reduce error: before={} after={}",
            before,
            after
        );
    }

    #[test]
    fn export_import_roundtrip() {
        let net = NeuralScheduler::new();
        let (h, o) = net.export_weights();
        let mut other = NeuralScheduler::new();
        // Force different init then restore.
        other.reset();
        other.import_weights(&h, &o);
        let w1 = net.get_output_weights();
        let w2 = other.get_output_weights();
        for i in 0..8 {
            assert!(fabs(w1[i] - w2[i]) < 1e-4);
        }
    }
}
