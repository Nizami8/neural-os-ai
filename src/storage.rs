/// Persistent storage for neural network weights
/// Saves/loads learned weights across system restarts

pub struct PersistentStorage {
    pub buffer: [u32; 32],  // 32 x 32-bit = 128 bytes для весов
}

impl PersistentStorage {
    pub fn new() -> Self {
        PersistentStorage {
            buffer: [0; 32],
        }
    }

    /// Сохраняет веса нейросети в буфер (в реальном OS это была бы запись в EEPROM/Flash)
    pub fn save_weights(&mut self, weights: &[[f32; 7]; 8], output_weights: &[f32; 8]) {
        let mut idx = 0;
        
        // Сохраняем входные веса (8 нейронов × 7 входов)
        for i in 0..8 {
            for j in 0..7 {
                if idx < 32 {
                    self.buffer[idx] = weights[i][j].to_bits();
                    idx += 1;
                }
            }
        }
        
        // Сохраняем выходные веса (8 весов)
        for i in 0..8 {
            if idx < 32 {
                self.buffer[idx] = output_weights[i].to_bits();
                idx += 1;
            }
        }
    }

    /// Восстанавливает веса из буфера
    pub fn load_weights(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        // В реальности это сложнее, но для примера:
        let mut weights = Vec::new();
        let mut output_weights = Vec::new();
        
        let mut idx = 0;
        
        // Загружаем входные веса
        for _ in 0..56 {  // 8×7
            if idx < 32 {
                weights.push(f32::from_bits(self.buffer[idx]));
                idx += 1;
            }
        }
        
        // Загружаем выходные веса
        for _ in 0..8 {
            if idx < 32 {
                output_weights.push(f32::from_bits(self.buffer[idx]));
                idx += 1;
            }
        }
        
        if weights.len() > 0 && output_weights.len() > 0 {
            Some((weights, output_weights))
        } else {
            None
        }
    }

    /// Очищает хранилище
    pub fn clear(&mut self) {
        for i in 0..32 {
            self.buffer[i] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_a_zeroed_32_slot_buffer() {
        let storage = PersistentStorage::new();
        assert_eq!(storage.buffer, [0; 32]);
    }

    #[test]
    fn save_weights_truncates_hidden_weights_and_skips_output_weights() {
        let mut storage = PersistentStorage::new();
        let mut weights = [[0.0; 7]; 8];
        let mut next = 1.0;
        for row in &mut weights {
            for value in row {
                *value = next;
                next += 1.0;
            }
        }
        let output_weights = [100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0];

        storage.save_weights(&weights, &output_weights);

        // The 32-slot buffer truncates the 56 hidden weights before outputs.
        for (slot, value) in storage.buffer.iter().enumerate() {
            assert_eq!(*value, (slot as f32 + 1.0).to_bits());
        }
        assert!(!storage
            .buffer
            .iter()
            .any(|bits| output_weights.iter().any(|weight| *bits == weight.to_bits())));

        let round_trip: Vec<f32> = storage
            .buffer
            .iter()
            .map(|bits| f32::from_bits(*bits))
            .collect();
        assert_eq!(
            round_trip,
            (1..=32).map(|value| value as f32).collect::<Vec<_>>()
        );
    }

    #[test]
    fn load_weights_returns_none_before_and_after_saving() {
        let mut storage = PersistentStorage::new();
        assert!(storage.load_weights().is_none());

        let weights = [[1.0; 7]; 8];
        let output_weights = [2.0; 8];
        storage.save_weights(&weights, &output_weights);

        // Loading exhausts the buffer on hidden weights before outputs are read.
        assert!(storage.load_weights().is_none());
    }

    #[test]
    fn clear_zeroes_saved_buffer() {
        let mut storage = PersistentStorage::new();
        storage.save_weights(&[[1.0; 7]; 8], &[2.0; 8]);
        assert_ne!(storage.buffer, [0; 32]);

        storage.clear();

        assert_eq!(storage.buffer, [0; 32]);
    }
}
