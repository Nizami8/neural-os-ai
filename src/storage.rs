/// Persistent storage for neural network weights
/// Saves/loads learned weights across system restarts

pub const HIDDEN_NEURONS: usize = 8;
pub const INPUT_WEIGHTS: usize = 7;

/// 8 нейронов × 7 входов + 8 выходных весов
pub const WEIGHT_COUNT: usize = HIDDEN_NEURONS * INPUT_WEIGHTS + HIDDEN_NEURONS;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    /// Буфер не вмещает все веса: сохранение отменено, чтобы не записать половину.
    BufferTooSmall { needed: usize, capacity: usize },
    /// Хранилище пустое (веса ни разу не сохранялись).
    Empty,
    /// В хранилище лежит битовый паттерн, который не является конечным числом.
    Corrupted { index: usize },
}

pub struct PersistentStorage {
    pub buffer: [u32; WEIGHT_COUNT],
    /// Сколько значений реально записано. 0 = хранилище пустое.
    len: usize,
}

impl PersistentStorage {
    pub fn new() -> Self {
        PersistentStorage {
            buffer: [0; WEIGHT_COUNT],
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        WEIGHT_COUNT
    }

    /// Сохраняет веса нейросети в буфер (в реальном OS это была бы запись в EEPROM/Flash).
    ///
    /// Возвращает `BufferTooSmall`, если веса не помещаются целиком: частичная
    /// запись не выполняется, поэтому предыдущее содержимое остается валидным.
    pub fn save_weights(
        &mut self,
        weights: &[[f32; INPUT_WEIGHTS]; HIDDEN_NEURONS],
        output_weights: &[f32; HIDDEN_NEURONS],
    ) -> Result<(), StorageError> {
        if WEIGHT_COUNT < weights.len() * INPUT_WEIGHTS + output_weights.len() {
            return Err(StorageError::BufferTooSmall {
                needed: weights.len() * INPUT_WEIGHTS + output_weights.len(),
                capacity: WEIGHT_COUNT,
            });
        }

        let mut idx = 0;

        for row in weights.iter() {
            for value in row.iter() {
                self.buffer[idx] = value.to_bits();
                idx += 1;
            }
        }

        for value in output_weights.iter() {
            self.buffer[idx] = value.to_bits();
            idx += 1;
        }

        self.len = idx;
        Ok(())
    }

    /// Восстанавливает веса из буфера.
    ///
    /// Ошибка возвращается, если хранилище пустое, содержит меньше значений, чем
    /// нужно сети, или хранит не-конечные значения (NaN/Inf сломали бы обучение).
    pub fn load_weights(
        &self,
    ) -> Result<([[f32; INPUT_WEIGHTS]; HIDDEN_NEURONS], [f32; HIDDEN_NEURONS]), StorageError> {
        if self.len == 0 {
            return Err(StorageError::Empty);
        }

        if self.len < WEIGHT_COUNT {
            return Err(StorageError::BufferTooSmall {
                needed: WEIGHT_COUNT,
                capacity: self.len,
            });
        }

        let mut weights = [[0.0f32; INPUT_WEIGHTS]; HIDDEN_NEURONS];
        let mut output_weights = [0.0f32; HIDDEN_NEURONS];
        let mut idx = 0;

        for i in 0..HIDDEN_NEURONS {
            for j in 0..INPUT_WEIGHTS {
                weights[i][j] = self.read_finite(idx)?;
                idx += 1;
            }
        }

        for i in 0..HIDDEN_NEURONS {
            output_weights[i] = self.read_finite(idx)?;
            idx += 1;
        }

        Ok((weights, output_weights))
    }

    fn read_finite(&self, index: usize) -> Result<f32, StorageError> {
        let value = f32::from_bits(self.buffer[index]);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(StorageError::Corrupted { index })
        }
    }

    /// Очищает хранилище
    pub fn clear(&mut self) {
        for slot in self.buffer.iter_mut() {
            *slot = 0;
        }
        self.len = 0;
    }
}
