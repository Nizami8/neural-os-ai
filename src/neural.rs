/// Advanced neural network with multiple layers and momentum
/// Multi-layer perceptron (MLP) with online learning

const HIDDEN_SIZE: usize = 8;
const INPUT_SIZE: usize = 7;  // расширенные метрики
const OUTPUT_SIZE: usize = 1;

pub struct NeuralScheduler {
    // Input → Hidden layer
    pub hidden_weights: [[f32; INPUT_SIZE]; HIDDEN_SIZE],
    pub hidden_bias: [f32; HIDDEN_SIZE],
    
    // Hidden → Output layer
    pub output_weights: [f32; HIDDEN_SIZE],
    pub output_bias: f32,
    
    // Momentum для обоих слоев
    pub hidden_velocity: [[f32; INPUT_SIZE]; HIDDEN_SIZE],
    pub output_velocity: [f32; HIDDEN_SIZE],
    
    pub learning_rate: f32,
    pub momentum: f32,
}

/// Расширенная статистика задачи
#[derive(Clone, Copy)]
pub struct TaskMetrics {
    pub task_id: usize,
    pub execution_time: u32,        // количество циклов
    pub wait_time: u32,             // сколько ждала в очереди
    pub memory_used: usize,
    pub ticks_since_run: u32,
    pub io_wait_count: u32,         // НОВОЕ: ожидания I/O
    pub context_switches: u32,      // НОВОЕ: кол-во переключений
    pub priority_boost: i32,        // НОВОЕ: ручной бустер
    pub ema_exec_time: f32,         // Exponential moving average
    pub priority: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TaskClass {
    RealTime,      // hard deadline
    Interactive,   // user-facing
    Batch,         // background
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
        let alpha = 0.3;  // вес недавних значений
        self.ema_exec_time = alpha * (new_exec_time as f32) 
                           + (1.0 - alpha) * self.ema_exec_time;
    }
}

impl NeuralScheduler {
    pub fn new() -> Self {
        let mut scheduler = NeuralScheduler {
            hidden_weights: [[0.5; INPUT_SIZE]; HIDDEN_SIZE],
            hidden_bias: [0.1; HIDDEN_SIZE],
            output_weights: [0.3; HIDDEN_SIZE],
            output_bias: 0.0,
            
            hidden_velocity: [[0.0; INPUT_SIZE]; HIDDEN_SIZE],
            output_velocity: [0.0; HIDDEN_SIZE],
            
            learning_rate: 0.01,
            momentum: 0.9,
        };
        
        // Инициализируем с малыми случайными значениями
        for i in 0..HIDDEN_SIZE {
            for j in 0..INPUT_SIZE {
                scheduler.hidden_weights[i][j] = 0.5 + (i as f32 * j as f32) % 0.1;
            }
            scheduler.output_weights[i] = 0.3 + (i as f32) % 0.1;
        }
        
        scheduler
    }

    /// Нормализация входов в диапазон [0, 1]
    fn normalize_inputs(&self, metrics: &TaskMetrics) -> [f32; INPUT_SIZE] {
        [
            (metrics.execution_time as f32) / 1000.0,
            (metrics.wait_time as f32) / 1000.0,
            (metrics.memory_used as f32) / 100.0,
            (metrics.ticks_since_run as f32) / 100.0,
            (metrics.io_wait_count as f32) / 100.0,
            (metrics.context_switches as f32) / 50.0,
            (metrics.priority_boost as f32) / 10.0,
        ]
    }

    /// ReLU активация для скрытого слоя
    #[inline]
    fn relu(x: f32) -> f32 {
        if x > 0.0 { x } else { 0.01 * x }  // Leaky ReLU
    }

    /// Производная ReLU
    #[inline]
    fn relu_derivative(x: f32) -> f32 {
        if x > 0.0 { 1.0 } else { 0.01 }
    }

    /// Forward pass: вычисляем скрытый слой
    fn forward_hidden(&self, inputs: &[f32; INPUT_SIZE]) -> [f32; HIDDEN_SIZE] {
        let mut hidden = [0.0; HIDDEN_SIZE];
        
        for i in 0..HIDDEN_SIZE {
            let mut z = self.hidden_bias[i];
            for j in 0..INPUT_SIZE {
                z += self.hidden_weights[i][j] * inputs[j];
            }
            hidden[i] = Self::relu(z);
        }
        
        hidden
    }

    /// Forward pass: выходной слой с Sigmoid
    fn forward_output(&self, hidden: &[f32; HIDDEN_SIZE]) -> f32 {
        let mut z = self.output_bias;
        for i in 0..HIDDEN_SIZE {
            z += self.output_weights[i] * hidden[i];
        }
        sigmoid(z)
    }

    /// Полный forward pass для предсказания приоритета
    pub fn predict_priority(&self, metrics: &TaskMetrics) -> f32 {
        let inputs = self.normalize_inputs(metrics);
        let hidden = self.forward_hidden(&inputs);
        self.forward_output(&hidden)
    }

    /// Обратное распространение с Momentum (SGD + Momentum)
    pub fn learn(&mut self, metrics: &TaskMetrics, target: f32) {
        let inputs = self.normalize_inputs(metrics);
        
        // Forward pass
        let hidden = self.forward_hidden(&inputs);
        let output = self.forward_output(&hidden);
        
        // Вычисляем ошибку
        let output_error = target - output;
        
        // Если ошибка слишком мала, не обновляем
        if output_error.abs() < 0.001 {
            return;
        }

        // Backprop: градиент выходного слоя
        let output_delta = output_error * sigmoid_derivative(output);
        
        // Обновляем выходной слой с momentum
        for i in 0..HIDDEN_SIZE {
            let grad = output_delta * hidden[i];
            self.output_velocity[i] = self.momentum * self.output_velocity[i]
                                    + self.learning_rate * grad;
            self.output_weights[i] += self.output_velocity[i];
        }
        self.output_bias += self.learning_rate * output_delta;
        
        // Backprop: градиент скрытого слоя
        for i in 0..HIDDEN_SIZE {
            let hidden_delta = output_delta * self.output_weights[i] 
                             * Self::relu_derivative(hidden[i]);
            
            for j in 0..INPUT_SIZE {
                let grad = hidden_delta * inputs[j];
                self.hidden_velocity[i][j] = self.momentum * self.hidden_velocity[i][j]
                                           + self.learning_rate * grad;
                self.hidden_weights[i][j] += self.hidden_velocity[i][j];
            }
            self.hidden_bias[i] += self.learning_rate * hidden_delta;
        }
    }

    /// Получить текущие выходные веса (для отладки)
    pub fn get_output_weights(&self) -> [f32; HIDDEN_SIZE] {
        self.output_weights
    }

    /// Сбросить веса на начальные значения
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Sigmoid активационная функция
#[inline]
pub fn sigmoid(x: f32) -> f32 {
    if x >= 0.0 {
        let value = exp(-x);
        if value == 0.0 {
            1.0 - f32::EPSILON
        } else {
            1.0 / (1.0 + value)
        }
    } else {
        let value = exp(x);
        if value == 0.0 {
            f32::MIN_POSITIVE
        } else {
            value / (1.0 + value)
        }
    }
}

/// Производная сигмоида для обратного распространения
#[inline]
pub fn sigmoid_derivative(y: f32) -> f32 {
    y * (1.0 - y)
}

/// Приблизительная экспоненциальная функция для embedded
#[inline]
pub fn exp(x: f32) -> f32 {
    if x > 10.0 {
        return 22026.0;
    }
    if x < -10.0 {
        return 0.0;
    }
    if x < 0.0 {
        return 1.0 / exp(-x);
    }

    let mut result = 1.0;
    let mut term = 1.0;
    
    for i in 1..=20 {
        term *= x / (i as f32);
        result += term;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx(actual: f32, expected: f32, tolerance: f32) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual} (tolerance {tolerance})"
        );
    }

    #[test]
    fn exp_handles_clamps_known_values_and_is_monotonic() {
        assert_eq!(exp(0.0), 1.0);
        assert_eq!(exp(11.0), 22026.0);
        assert_eq!(exp(-11.0), 0.0);
        assert_approx(exp(1.0), std::f32::consts::E, 0.001);
        assert_approx(exp(-2.0), (-2.0f32).exp(), 0.001);

        let values = [-10.0, -5.0, -1.0, 0.0, 1.0, 5.0, 10.0];
        for pair in values.windows(2) {
            assert!(exp(pair[1]) > exp(pair[0]));
        }
    }

    #[test]
    fn sigmoid_is_bounded_symmetric_and_monotonic() {
        assert_eq!(sigmoid(0.0), 0.5);

        let values = [-50.0, -20.0, -10.0, -1.0, 0.0, 1.0, 10.0, 20.0, 50.0];
        for pair in values.windows(2) {
            let left = sigmoid(pair[0]);
            let right = sigmoid(pair[1]);
            assert!(left > 0.0 && left < 1.0);
            assert!(left <= right);
            assert!(right > 0.0 && right < 1.0);
        }

        for x in [-50.0, -10.0, -2.0, 0.0, 2.0, 10.0, 50.0] {
            assert_approx(sigmoid(-x), 1.0 - sigmoid(x), 0.0001);
        }
    }

    #[test]
    fn sigmoid_derivative_has_expected_values() {
        assert_approx(sigmoid_derivative(0.5), 0.25, f32::EPSILON);
        assert_eq!(sigmoid_derivative(0.0), 0.0);
        assert_eq!(sigmoid_derivative(1.0), 0.0);
    }

    #[test]
    fn leaky_relu_and_derivative_cover_both_sides_and_boundary() {
        assert_eq!(NeuralScheduler::relu(2.5), 2.5);
        assert_eq!(NeuralScheduler::relu_derivative(2.5), 1.0);
        assert_approx(NeuralScheduler::relu(-2.5), -0.025, 0.000001);
        assert_eq!(NeuralScheduler::relu_derivative(-2.5), 0.01);
        assert_eq!(NeuralScheduler::relu(0.0), 0.0);
        assert_eq!(NeuralScheduler::relu_derivative(0.0), 0.01);
    }

    #[test]
    fn task_metrics_new_initializes_all_fields() {
        let metrics = TaskMetrics::new(42);
        assert_eq!(metrics.task_id, 42);
        assert_eq!(metrics.execution_time, 0);
        assert_eq!(metrics.wait_time, 0);
        assert_eq!(metrics.memory_used, 0);
        assert_eq!(metrics.ticks_since_run, 0);
        assert_eq!(metrics.io_wait_count, 0);
        assert_eq!(metrics.context_switches, 0);
        assert_eq!(metrics.priority_boost, 0);
        assert_eq!(metrics.ema_exec_time, 0.0);
        assert_eq!(metrics.priority, 0.0);
    }

    #[test]
    fn update_ema_uses_alpha_and_converges() {
        let mut metrics = TaskMetrics::new(1);
        metrics.update_ema(100);
        assert_approx(metrics.ema_exec_time, 30.0, 0.00001);

        for _ in 0..100 {
            metrics.update_ema(100);
        }
        assert!((100.0 - metrics.ema_exec_time).abs() < 0.001);
    }

    #[test]
    fn normalize_inputs_uses_documented_divisors() {
        let mut metrics = TaskMetrics::new(1);
        metrics.execution_time = 250;
        metrics.wait_time = 500;
        metrics.memory_used = 75;
        metrics.ticks_since_run = 25;
        metrics.io_wait_count = 40;
        metrics.context_switches = 10;
        metrics.priority_boost = -5;

        assert_eq!(
            NeuralScheduler::new().normalize_inputs(&metrics),
            [0.25, 0.5, 0.75, 0.25, 0.4, 0.2, -0.5]
        );
    }

    #[test]
    fn new_uses_expected_initial_weights_and_zero_velocities() {
        let scheduler = NeuralScheduler::new();
        assert_eq!(scheduler.learning_rate, 0.01);
        assert_eq!(scheduler.momentum, 0.9);
        assert_eq!(scheduler.hidden_bias, [0.1; HIDDEN_SIZE]);
        assert_eq!(scheduler.output_bias, 0.0);
        assert_eq!(scheduler.hidden_velocity, [[0.0; INPUT_SIZE]; HIDDEN_SIZE]);
        assert_eq!(scheduler.output_velocity, [0.0; HIDDEN_SIZE]);

        for i in 0..HIDDEN_SIZE {
            for j in 0..INPUT_SIZE {
                assert_eq!(
                    scheduler.hidden_weights[i][j],
                    0.5 + (i as f32 * j as f32) % 0.1
                );
            }
            assert_eq!(scheduler.output_weights[i], 0.3 + (i as f32) % 0.1);
        }
    }

    #[test]
    fn forward_passes_are_deterministic_and_finite() {
        let scheduler = NeuralScheduler::new();
        let metrics = TaskMetrics::new(1);
        let inputs = scheduler.normalize_inputs(&metrics);
        let hidden = scheduler.forward_hidden(&inputs);
        assert_eq!(hidden, scheduler.forward_hidden(&inputs));
        assert!(hidden.iter().all(|value| value.is_finite()));

        let output = scheduler.forward_output(&hidden);
        assert_eq!(output, scheduler.forward_output(&hidden));
        assert!(output.is_finite());
        assert!(scheduler.predict_priority(&metrics) > 0.0);
        assert!(scheduler.predict_priority(&metrics) < 1.0);

        let mut large_metrics = TaskMetrics::new(2);
        large_metrics.execution_time = u32::MAX;
        large_metrics.wait_time = u32::MAX;
        large_metrics.memory_used = usize::MAX;
        large_metrics.ticks_since_run = u32::MAX;
        large_metrics.io_wait_count = u32::MAX;
        large_metrics.context_switches = u32::MAX;
        large_metrics.priority_boost = i32::MAX;
        let large_prediction = scheduler.predict_priority(&large_metrics);
        assert!(large_prediction.is_finite());
        assert!(large_prediction > 0.0 && large_prediction < 1.0);
    }

    #[test]
    fn learn_moves_prediction_toward_targets_and_uses_momentum() {
        let mut metrics = TaskMetrics::new(1);
        metrics.execution_time = 100;
        metrics.wait_time = 50;
        metrics.memory_used = 25;

        let mut upward = NeuralScheduler::new();
        let initial_upward = upward.predict_priority(&metrics);
        for _ in 0..5 {
            upward.learn(&metrics, 1.0);
        }
        assert!(upward.predict_priority(&metrics) > initial_upward);
        assert!(upward.output_velocity.iter().any(|value| *value != 0.0));
        assert!(upward
            .hidden_velocity
            .iter()
            .flatten()
            .any(|value| *value != 0.0));

        let mut downward = NeuralScheduler::new();
        let initial_downward = downward.predict_priority(&metrics);
        for _ in 0..5 {
            downward.learn(&metrics, 0.0);
        }
        assert!(downward.predict_priority(&metrics) < initial_downward);
    }

    #[test]
    fn learn_returns_without_changes_when_prediction_matches_target() {
        let metrics = TaskMetrics::new(1);
        let mut scheduler = NeuralScheduler::new();
        let target = scheduler.predict_priority(&metrics);
        let hidden_weights = scheduler.hidden_weights;
        let hidden_bias = scheduler.hidden_bias;
        let output_weights = scheduler.output_weights;
        let output_bias = scheduler.output_bias;
        let hidden_velocity = scheduler.hidden_velocity;
        let output_velocity = scheduler.output_velocity;

        scheduler.learn(&metrics, target);

        assert_eq!(scheduler.hidden_weights, hidden_weights);
        assert_eq!(scheduler.hidden_bias, hidden_bias);
        assert_eq!(scheduler.output_weights, output_weights);
        assert_eq!(scheduler.output_bias, output_bias);
        assert_eq!(scheduler.hidden_velocity, hidden_velocity);
        assert_eq!(scheduler.output_velocity, output_velocity);
    }

    #[test]
    fn reset_restores_a_fresh_scheduler() {
        let metrics = TaskMetrics::new(1);
        let mut scheduler = NeuralScheduler::new();
        scheduler.learn(&metrics, 1.0);
        scheduler.reset();
        let fresh = NeuralScheduler::new();

        assert_eq!(scheduler.hidden_weights, fresh.hidden_weights);
        assert_eq!(scheduler.hidden_bias, fresh.hidden_bias);
        assert_eq!(scheduler.output_weights, fresh.output_weights);
        assert_eq!(scheduler.output_bias, fresh.output_bias);
        assert_eq!(scheduler.hidden_velocity, fresh.hidden_velocity);
        assert_eq!(scheduler.output_velocity, fresh.output_velocity);
        assert_eq!(scheduler.learning_rate, fresh.learning_rate);
        assert_eq!(scheduler.momentum, fresh.momentum);
    }

    #[test]
    fn get_output_weights_reflects_learning_updates() {
        let metrics = TaskMetrics::new(1);
        let mut scheduler = NeuralScheduler::new();
        let before = scheduler.get_output_weights();
        scheduler.learn(&metrics, 1.0);
        let after = scheduler.get_output_weights();

        assert_eq!(after, scheduler.output_weights);
        assert_ne!(after, before);
    }

    #[test]
    fn learn_with_extreme_metrics_keeps_weights_finite() {
        let mut metrics = TaskMetrics::new(1);
        metrics.execution_time = u32::MAX;
        metrics.wait_time = u32::MAX;
        metrics.memory_used = usize::MAX;
        metrics.ticks_since_run = u32::MAX;
        metrics.io_wait_count = u32::MAX;
        metrics.context_switches = u32::MAX;
        metrics.priority_boost = i32::MAX;

        let mut scheduler = NeuralScheduler::new();
        scheduler.learn(&metrics, 0.0);

        assert!(scheduler
            .hidden_weights
            .iter()
            .flatten()
            .all(|value| value.is_finite()));
        assert!(scheduler.hidden_bias.iter().all(|value| value.is_finite()));
        assert!(scheduler.output_weights.iter().all(|value| value.is_finite()));
        assert!(scheduler.output_bias.is_finite());
        assert!(scheduler
            .hidden_velocity
            .iter()
            .flatten()
            .all(|value| value.is_finite()));
        assert!(scheduler.output_velocity.iter().all(|value| value.is_finite()));
    }
}
