/// Simple neural network for task priority prediction
/// Uses online learning (weight updates during OS runtime)

pub struct NeuralScheduler {
    // Weights для простой сети: [w0, w1, w2, w3]
    pub weights: [f32; 4],
    
    // Bias
    pub bias: f32,
    
    // Learning rate
    pub learning_rate: f32,
}

/// Собранная статистика задачи для обучения
#[derive(Clone, Copy)]
pub struct TaskMetrics {
    pub task_id: usize,
    pub execution_time: u32,      // количество циклов
    pub wait_time: u32,           // сколько ждала в очереди
    pub memory_used: usize,       // примерно (не точно)
    pub ticks_since_run: u32,     // когда последний раз выполнялась
    pub priority: f32,            // вычисленный приоритет
}

impl NeuralScheduler {
    pub fn new() -> Self {
        NeuralScheduler {
            weights: [0.5, 0.3, 0.2, 0.1],
            bias: 0.0,
            learning_rate: 0.01,
        }
    }

    /// Предсказываем приоритет задачи на основе метрик
    pub fn predict_priority(&self, metrics: &TaskMetrics) -> f32 {
        // Нормализуем входные данные
        let exec_norm = (metrics.execution_time as f32) / 1000.0; // 0-1 range
        let wait_norm = (metrics.wait_time as f32) / 1000.0;
        let mem_norm = (metrics.memory_used as f32) / 100.0;
        let ticks_norm = (metrics.ticks_since_run as f32) / 100.0;

        // Простая линейная модель с сигмоидой
        let z = self.weights[0] * exec_norm
            + self.weights[1] * wait_norm
            + self.weights[2] * mem_norm
            + self.weights[3] * ticks_norm
            + self.bias;

        // Sigmoid функция для вывода в диапазон (0, 1)
        sigmoid(z)
    }

    /// Обновляем веса на основе ошибки (feedback)
    /// target: ожидаемый приоритет (например, 1.0 если задача должна была работать)
    /// prediction: что предсказала сеть
    pub fn learn(&mut self, metrics: &TaskMetrics, target: f32) {
        let prediction = self.predict_priority(metrics);
        let error = target - prediction;

        // Если ошибка слишком мала, не обновляем
        if error.abs() < 0.01 {
            return;
        }

        // Нормализуем входные данные (как в predict)
        let exec_norm = (metrics.execution_time as f32) / 1000.0;
        let wait_norm = (metrics.wait_time as f32) / 1000.0;
        let mem_norm = (metrics.memory_used as f32) / 100.0;
        let ticks_norm = (metrics.ticks_since_run as f32) / 100.0;

        // Gradient descent: обновляем веса
        let delta = error * sigmoid_derivative(self.predict_priority(metrics));
        
        self.weights[0] += self.learning_rate * delta * exec_norm;
        self.weights[1] += self.learning_rate * delta * wait_norm;
        self.weights[2] += self.learning_rate * delta * mem_norm;
        self.weights[3] += self.learning_rate * delta * ticks_norm;
        self.bias += self.learning_rate * delta;
    }

    /// Получить текущие веса (для отладки)
    pub fn get_weights(&self) -> [f32; 4] {
        self.weights
    }

    /// Сбросить веса на начальные значения
    pub fn reset(&mut self) {
        self.weights = [0.5, 0.3, 0.2, 0.1];
        self.bias = 0.0;
    }
}

/// Sigmoid активационная функция
#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + exp(-x))
}

/// Производная сигмоида для обратного распространения
#[inline]
fn sigmoid_derivative(y: f32) -> f32 {
    y * (1.0 - y)
}

/// Приблизительная экспоненциальная функция для embedded
#[inline]
fn exp(x: f32) -> f32 {
    // Быстрая аппроксимация e^x для небольших значений
    // Используем разложение Тейлора: e^x ≈ 1 + x + x²/2 + x³/6
    if x > 10.0 {
        return 22026.0; // e^10
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid() {
        assert!(sigmoid(0.0) > 0.49 && sigmoid(0.0) < 0.51); // sigmoid(0) ≈ 0.5
        assert!(sigmoid(10.0) > 0.99); // sigmoid(10) ≈ 1.0
        assert!(sigmoid(-10.0) < 0.01); // sigmoid(-10) ≈ 0.0
    }

    #[test]
    fn test_neural_scheduler() {
        let scheduler = NeuralScheduler::new();
        let metrics = TaskMetrics {
            task_id: 1,
            execution_time: 100,
            wait_time: 50,
            memory_used: 10,
            ticks_since_run: 5,
            priority: 0.0,
        };

        let priority = scheduler.predict_priority(&metrics);
        assert!(priority >= 0.0 && priority <= 1.0);
    }
}
