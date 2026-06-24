use crate::scheduler::{Scheduler, TaskState};
use crate::neural::{NeuralScheduler, TaskMetrics};

pub const METRICS_HISTORY_SIZE: usize = 64;

pub struct MetricsCollector {
    pub history: [Option<TaskMetrics>; METRICS_HISTORY_SIZE],
    pub index: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        MetricsCollector {
            history: [None; METRICS_HISTORY_SIZE],
            index: 0,
        }
    }

    /// Записать метрику задачи
    pub fn record(&mut self, metrics: TaskMetrics) {
        self.history[self.index] = Some(metrics);
        self.index = (self.index + 1) % METRICS_HISTORY_SIZE;
    }

    /// Получить среднюю метрику для задачи
    pub fn get_average(&self, task_id: usize) -> Option<TaskMetrics> {
        let mut count = 0;
        let mut avg_exec = 0u32;
        let mut avg_wait = 0u32;
        let mut avg_mem = 0usize;
        let mut avg_ticks = 0u32;

        for entry in self.history.iter() {
            if let Some(metrics) = entry {
                if metrics.task_id == task_id {
                    avg_exec = avg_exec.saturating_add(metrics.execution_time);
                    avg_wait = avg_wait.saturating_add(metrics.wait_time);
                    avg_mem = avg_mem.saturating_add(metrics.memory_used);
                    avg_ticks = avg_ticks.saturating_add(metrics.ticks_since_run);
                    count += 1;
                }
            }
        }

        if count > 0 {
            Some(TaskMetrics {
                task_id,
                execution_time: avg_exec / count,
                wait_time: avg_wait / count,
                memory_used: avg_mem / count,
                ticks_since_run: avg_ticks / count,
                priority: 0.0,
            })
        } else {
            None
        }
    }

    /// Получить количество записей для задачи
    pub fn count_for_task(&self, task_id: usize) -> usize {
        self.history.iter().filter(|m| m.is_some() && m.unwrap().task_id == task_id).count()
    }
}

pub struct AdaptiveScheduler {
    pub base_scheduler: Scheduler,
    pub neural: NeuralScheduler,
    pub metrics: MetricsCollector,
    pub last_execution_tick: [u32; 4],
}

impl AdaptiveScheduler {
    pub fn new() -> Self {
        AdaptiveScheduler {
            base_scheduler: Scheduler::new(),
            neural: NeuralScheduler::new(),
            metrics: MetricsCollector::new(),
            last_execution_tick: [0; 4],
        }
    }

    /// Добавить задачу (как в базовом scheduler)
    pub fn add_task(&mut self, id: usize, entry: fn()) {
        self.base_scheduler.add_task(id, entry);
    }

    /// Найти задачу с наивысшим предсказанным приоритетом
    pub fn select_next_task(&mut self) -> usize {
        let current_tick = self.base_scheduler.tick as u32;
        let mut best_priority = -1.0f32;
        let mut best_idx = self.base_scheduler.current;

        for i in 0..4 {
            if let Some(task) = &self.base_scheduler.tasks[i] {
                if task.state == TaskState::Ready {
                    // Собираем метрики для этой задачи
                    let metrics = TaskMetrics {
                        task_id: task.id,
                        execution_time: 100, // placeholder
                        wait_time: current_tick.saturating_sub(self.last_execution_tick[i]),
                        memory_used: 0,
                        ticks_since_run: current_tick.saturating_sub(self.last_execution_tick[i]),
                        priority: 0.0,
                    };

                    // Предсказываем приоритет
                    let priority = self.neural.predict_priority(&metrics);

                    if priority > best_priority {
                        best_priority = priority;
                        best_idx = i;
                    }
                }
            }
        }

        best_idx
    }

    /// Адаптивное планирование с обучением
    pub fn adaptive_schedule(&mut self) {
        self.base_scheduler.tick += 1;
        let current_tick = self.base_scheduler.tick as u32;

        // Выбираем следующую задачу на основе нейросети
        let next_idx = self.select_next_task();

        // Переключаемся на выбранную задачу
        if next_idx != self.base_scheduler.current {
            if let (Some(old_task), Some(new_task)) = (
                self.base_scheduler.tasks[self.base_scheduler.current].as_mut(),
                self.base_scheduler.tasks[next_idx].as_mut(),
            ) {
                old_task.state = TaskState::Ready;
                new_task.state = TaskState::Running;

                // Собираем метрики о новой задаче
                let metrics = TaskMetrics {
                    task_id: new_task.id,
                    execution_time: 100,
                    wait_time: current_tick.saturating_sub(self.last_execution_tick[next_idx]),
                    memory_used: 0,
                    ticks_since_run: current_tick.saturating_sub(self.last_execution_tick[next_idx]),
                    priority: 0.0,
                };

                self.metrics.record(metrics);
                self.last_execution_tick[next_idx] = current_tick;

                // Online learning: если задача часто выбирается, она должна иметь высокий приоритет
                let target = if metrics.ticks_since_run > 50 { 1.0 } else { 0.7 };
                self.neural.learn(&metrics, target);

                self.base_scheduler.current = next_idx;

                // Переключаем контекст
                unsafe {
                    crate::scheduler::context_switch(&mut old_task.context, &mut new_task.context);
                }
            }
        }
    }

    /// Получить текущие веса нейросети
    pub fn get_neural_weights(&self) -> [f32; 4] {
        self.neural.get_weights()
    }

    /// Сбросить обучение
    pub fn reset_learning(&mut self) {
        self.neural.reset();
        self.metrics = MetricsCollector::new();
    }
}
