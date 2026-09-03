use crate::scheduler::{Scheduler, TaskState, MAX_TASKS};
use crate::neural::{NeuralScheduler, TaskMetrics, TaskClass};

pub const METRICS_HISTORY_SIZE: usize = 128;
pub const QUANTUM: u32 = 100;  // 10ms в тиках

#[derive(Clone, Copy, PartialEq)]
pub enum SchedulingStrategy {
    NeuralOnly,           // чистая нейросеть
    LoadBalanced,         // neural + fairness
    PredictivePreempt,    // с предсказанием
}

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

    pub fn record(&mut self, metrics: TaskMetrics) {
        self.history[self.index] = Some(metrics);
        self.index = (self.index + 1) % METRICS_HISTORY_SIZE;
    }

    pub fn get_average(&self, task_id: usize) -> Option<TaskMetrics> {
        let mut count: u32 = 0;
        let mut avg_exec = 0u32;
        let mut avg_wait = 0u32;
        let mut avg_mem = 0usize;
        let mut avg_ticks = 0u32;
        let mut avg_io = 0u32;
        let mut avg_ctx = 0u32;

        for entry in self.history.iter() {
            if let Some(metrics) = entry {
                if metrics.task_id == task_id {
                    avg_exec = avg_exec.saturating_add(metrics.execution_time);
                    avg_wait = avg_wait.saturating_add(metrics.wait_time);
                    avg_mem = avg_mem.saturating_add(metrics.memory_used);
                    avg_ticks = avg_ticks.saturating_add(metrics.ticks_since_run);
                    avg_io = avg_io.saturating_add(metrics.io_wait_count);
                    avg_ctx = avg_ctx.saturating_add(metrics.context_switches);
                    count += 1;
                }
            }
        }

        if count > 0 {
            Some(TaskMetrics {
                task_id,
                execution_time: avg_exec / count,
                wait_time: avg_wait / count,
                memory_used: avg_mem / count as usize,
                ticks_since_run: avg_ticks / count,
                io_wait_count: avg_io / count,
                context_switches: avg_ctx / count,
                priority_boost: 0,
                ema_exec_time: 0.0,
                priority: 0.0,
            })
        } else {
            None
        }
    }

    pub fn count_for_task(&self, task_id: usize) -> usize {
        self.history.iter().filter(|m| m.is_some() && m.unwrap().task_id == task_id).count()
    }
}

pub struct OSStatistics {
    pub total_context_switches: u64,
    pub total_preemptions: u64,
    pub missed_deadlines: u32,
    pub avg_wait_time: f32,
    pub cpu_utilization: f32,
    pub fairness_index: f32,  // 0-1, близко к 1 = справедливо
}

pub struct RewardSignal {
    pub task_id: usize,
    pub reward: f32,  // +1.0 = успех, -1.0 = failed deadline, 0.5 = normal
}

pub struct AdaptiveScheduler {
    pub base_scheduler: Scheduler,
    pub neural: NeuralScheduler,
    pub metrics: MetricsCollector,
    pub last_execution_tick: [u32; MAX_TASKS],
    pub task_classes: [TaskClass; MAX_TASKS],
    pub task_deadlines: [u32; MAX_TASKS],
    pub execution_start_time: [u32; MAX_TASKS],
    pub total_exec_time: [u32; MAX_TASKS],  // для fairness
    
    // Статистика
    pub stats: OSStatistics,
    pub strategy: SchedulingStrategy,
    
    // Q-learning для reinforcement
    pub q_values: [[f32; 4]; MAX_TASKS],  // [task][action]
    pub q_learning_rate: f32,
    pub q_discount: f32,
}

impl AdaptiveScheduler {
    pub fn new() -> Self {
        AdaptiveScheduler {
            base_scheduler: Scheduler::new(),
            neural: NeuralScheduler::new(),
            metrics: MetricsCollector::new(),
            last_execution_tick: [0; MAX_TASKS],
            task_classes: [TaskClass::Batch; MAX_TASKS],
            task_deadlines: [0; MAX_TASKS],
            execution_start_time: [0; MAX_TASKS],
            total_exec_time: [0; MAX_TASKS],
            
            stats: OSStatistics {
                total_context_switches: 0,
                total_preemptions: 0,
                missed_deadlines: 0,
                avg_wait_time: 0.0,
                cpu_utilization: 0.0,
                fairness_index: 1.0,
            },
            strategy: SchedulingStrategy::LoadBalanced,
            
            q_values: [[0.0; 4]; MAX_TASKS],
            q_learning_rate: 0.1,
            q_discount: 0.9,
        }
    }

    pub fn add_task(&mut self, id: usize, entry: fn(), task_class: TaskClass) {
        if id == 0 || id > MAX_TASKS {
            return;
        }
        self.base_scheduler.add_task(id, entry);
        self.task_classes[id - 1] = task_class;
    }

    /// Grant a task (by id) an IPC capability on an endpoint.
    pub fn grant_cap(&mut self, id: usize, endpoint: usize, rights: u8) {
        self.base_scheduler.grant(id, endpoint, rights);
    }

    /// Load-balanced selection с fairness
    pub fn select_next_task_balanced(&mut self) -> usize {
        let current_tick = self.base_scheduler.tick as u32;
        let mut best_priority = -1.0f32;
        let mut best_idx = self.base_scheduler.current;
        
        // Вычисляем среднее время выполнения
        let mut total_time = 0u32;
        let mut count = 0;
        for i in 0..MAX_TASKS {
            if self.base_scheduler.tasks[i].is_some() {
                total_time = total_time.saturating_add(self.total_exec_time[i]);
                count += 1;
            }
        }
        
        let avg_time = if count > 0 { total_time / count } else { 1 };
        let fair_share = if count > 0 { 1.0 / (count as f32) } else { 0.25 };

        for i in 0..MAX_TASKS {
            if let Some(task) = &self.base_scheduler.tasks[i] {
                if task.state == TaskState::Ready {
                    let mut metrics = TaskMetrics::new(task.id);
                    metrics.execution_time = self.total_exec_time[i];
                    metrics.wait_time = current_tick.saturating_sub(self.last_execution_tick[i]);
                    metrics.ticks_since_run = metrics.wait_time;
                    metrics.context_switches = self.stats.total_context_switches as u32;
                    
                    let mut priority = self.neural.predict_priority(&metrics);
                    
                    // RealTime boost
                    if self.task_classes[i] == TaskClass::RealTime {
                        if self.task_deadlines[i] > 0 && self.task_deadlines[i] < QUANTUM / 2 {
                            priority += 2.0;  // Высокий приоритет перед deadline
                        }
                    }
                    
                    // Fairness penalty
                    let usage_ratio = (self.total_exec_time[i] as f32) / (avg_time as f32).max(1.0);
                    if usage_ratio > fair_share * 2.0 {
                        priority -= 0.5;  // Penalize если уже много выполнялась
                    }
                    
                    // Interactive boost
                    if self.task_classes[i] == TaskClass::Interactive {
                        if metrics.wait_time > 20 {
                            priority += 0.3;  // Low latency boost
                        }
                    }
                    
                    if priority > best_priority {
                        best_priority = priority;
                        best_idx = i;
                    }
                }
            }
        }

        best_idx
    }

    /// Предсказываем, когда надо переключиться
    pub fn predict_preemption(&self, current_idx: usize) -> bool {
        if current_idx >= self.base_scheduler.tasks.len()
            || current_idx >= self.task_classes.len()
            || current_idx >= self.execution_start_time.len()
            || current_idx >= self.task_deadlines.len()
        {
            return false;
        }

        let waiting_count = self.base_scheduler.tasks.iter()
            .filter(|t| t.is_some() && t.unwrap().state == TaskState::Ready)
            .count();
        
        let current_task = &self.base_scheduler.tasks[current_idx];
        if let Some(task) = current_task {
            let exec_time = self.execution_start_time[current_idx];
            
            // Преемптить если:
            // 1. Много задач ждут
            if waiting_count > 2 && exec_time > QUANTUM / 3 {
                return true;
            }
            
            // 2. Time quantum истек
            if exec_time > QUANTUM {
                return true;
            }
            
            // 3. RealTime задача с deadline
            if self.task_classes[current_idx] == TaskClass::RealTime {
                if self.task_deadlines[current_idx] > 0 && self.task_deadlines[current_idx] < 10 {
                    return true;
                }
            }
        }
        
        false
    }

    /// Q-learning с reward signal
    pub fn learn_with_reward(&mut self, signal: RewardSignal) {
        if signal.task_id == 0 || signal.task_id > MAX_TASKS {
            return;
        }
        let task_idx = signal.task_id - 1;
        
        // Обновляем Q-value для этой задачи
        let current_q = self.q_values[task_idx][0];
        let future_max_q = self.q_values[task_idx].iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        
        let new_q = current_q + self.q_learning_rate * 
                   (signal.reward + self.q_discount * future_max_q - current_q);
        
        self.q_values[task_idx][0] = new_q;
        
        // Также обновляем neural network
        let target = (signal.reward + 1.0) / 2.0;  // normalize to [0, 1]
        let mut metrics = TaskMetrics::new(signal.task_id);
        metrics.execution_time = self.total_exec_time[task_idx];
        self.neural.learn(&metrics, target);
    }

    /// Основное адаптивное расписание
    pub fn adaptive_schedule(&mut self) {
        self.base_scheduler.tick += 1;
        let current_tick = self.base_scheduler.tick as u32;

        // Обновляем deadlines
        for i in 0..MAX_TASKS {
            if self.task_deadlines[i] > 0 {
                self.task_deadlines[i] = self.task_deadlines[i].saturating_sub(1);
            }
        }

        // Выбираем следующую задачу в зависимости от стратегии
        let next_idx = match self.strategy {
            SchedulingStrategy::NeuralOnly => {
                let mut best_priority = -1.0f32;
                let mut best_idx = self.base_scheduler.current;
                for i in 0..MAX_TASKS {
                    if let Some(task) = &self.base_scheduler.tasks[i] {
                        if task.state == TaskState::Ready {
                            let metrics = TaskMetrics::new(task.id);
                            let priority = self.neural.predict_priority(&metrics);
                            if priority > best_priority {
                                best_priority = priority;
                                best_idx = i;
                            }
                        }
                    }
                }
                best_idx
            },
            SchedulingStrategy::LoadBalanced => self.select_next_task_balanced(),
            SchedulingStrategy::PredictivePreempt => {
                if self.predict_preemption(self.base_scheduler.current) {
                    self.select_next_task_balanced()
                } else {
                    self.base_scheduler.current
                }
            },
        };

        // Переключаемся на выбранную задачу
        let cur = self.base_scheduler.current;
        if next_idx != cur {
            // Получаем два непересекающихся изменяемых ссылки на задачи
            // (split_at_mut, т.к. индексировать массив дважды нельзя).
            let (old_slot, new_slot) = if cur < next_idx {
                let (left, right) = self.base_scheduler.tasks.split_at_mut(next_idx);
                (&mut left[cur], &mut right[0])
            } else {
                let (left, right) = self.base_scheduler.tasks.split_at_mut(cur);
                (&mut right[0], &mut left[next_idx])
            };

            if let (Some(old_task), Some(new_task)) = (old_slot.as_mut(), new_slot.as_mut()) {
                // Обновляем статистику
                let exec_time = current_tick.saturating_sub(self.execution_start_time[cur]);
                self.total_exec_time[cur] = self.total_exec_time[cur].saturating_add(exec_time);
                
                old_task.state = TaskState::Ready;
                new_task.state = TaskState::Running;

                let mut metrics = TaskMetrics::new(new_task.id);
                metrics.execution_time = exec_time;
                metrics.wait_time = current_tick.saturating_sub(self.last_execution_tick[next_idx]);
                metrics.ticks_since_run = metrics.wait_time;
                metrics.context_switches = (self.stats.total_context_switches as u32) % 255;

                self.metrics.record(metrics);
                self.last_execution_tick[next_idx] = current_tick;
                self.execution_start_time[next_idx] = current_tick;

                // Online learning
                let target = if metrics.wait_time > 50 { 1.0 } else { 0.7 };
                self.neural.learn(&metrics, target);

                self.base_scheduler.current = next_idx;
                self.stats.total_context_switches += 1;
                self.stats.total_preemptions += 1;

                unsafe {
                    crate::scheduler::context_switch(&mut old_task.context, &mut new_task.context);
                }
            }
        }
    }

    pub fn get_neural_weights(&self) -> [f32; 8] {
        self.neural.get_output_weights()
    }

    pub fn collect_statistics(&mut self) {
        let mut total_wait = 0u32;
        let mut wait_count = 0;
        
        for i in 0..MAX_TASKS {
            if self.base_scheduler.tasks[i].is_some() {
                let wait = (self.base_scheduler.tick as u32).saturating_sub(self.last_execution_tick[i]);
                total_wait = total_wait.saturating_add(wait);
                wait_count += 1;
            }
        }
        
        self.stats.avg_wait_time = if wait_count > 0 {
            (total_wait as f32) / (wait_count as f32)
        } else {
            0.0
        };
        
        // Fairness index (Jain's fairness index)
        let mut sum_sq = 0.0;
        let mut sum = 0.0;
        for i in 0..MAX_TASKS {
            if self.base_scheduler.tasks[i].is_some() {
                let time = (self.total_exec_time[i] as f32).max(1.0);
                sum_sq += time * time;
                sum += time;
            }
        }
        
        if sum > 0.0 {
            self.stats.fairness_index = (sum * sum) / (wait_count as f32 * sum_sq).max(1.0);
        }
    }

    pub fn reset_learning(&mut self) {
        self.neural.reset();
        self.metrics = MetricsCollector::new();
    }

    pub fn set_task_deadline(&mut self, task_id: usize, ticks: u32) {
        if task_id > 0 && task_id <= MAX_TASKS {
            self.task_deadlines[task_id - 1] = ticks;
        }
    }
}
