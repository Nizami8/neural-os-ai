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

    /// Метрики задачи, наблюдаемые планировщиком в момент переключения
    pub fn with_timing(
        task_id: usize,
        execution_time: u32,
        wait_time: u32,
        context_switches: u32,
    ) -> Self {
        let mut metrics = Self::new(task_id);
        metrics.execution_time = execution_time;
        metrics.wait_time = wait_time;
        metrics.ticks_since_run = wait_time;
        metrics.context_switches = context_switches;
        metrics
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
    1.0 / (1.0 + exp(-x))
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

    let mut result = 1.0;
    let mut term = 1.0;
    
    for i in 1..=20 {
        term *= x / (i as f32);
        result += term;
    }

    result
}
