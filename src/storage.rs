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
