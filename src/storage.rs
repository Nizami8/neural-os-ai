/// Persistent storage for neural network weights
/// Saves/loads learned weights across system restarts

pub struct PersistentStorage {
    pub buffer: [u32; 66],  // заголовок и 64 веса по 32 бита
}

impl PersistentStorage {
    pub fn new() -> Self {
        PersistentStorage {
            buffer: [0; 66],
        }
    }

    /// Сохраняет веса нейросети в буфер (в реальном OS это была бы запись в EEPROM/Flash)
    pub fn save_weights(&mut self, weights: &[[f32; 7]; 8], output_weights: &[f32; 8]) {
        const MAGIC_VERSION: u32 = 0x4E_4F_53_01;
        const VALID_MARKER: u32 = 0x5641_4C49;
        let mut idx = 2;

        // Сначала помечаем данные недействительными, чтобы незавершенная запись не загрузилась.
        self.buffer[1] = 0;
        
        // Сохраняем входные веса (8 нейронов × 7 входов)
        for i in 0..8 {
            for j in 0..7 {
                self.buffer[idx] = weights[i][j].to_bits();
                idx += 1;
            }
        }
        
        // Сохраняем выходные веса (8 весов)
        for i in 0..8 {
            self.buffer[idx] = output_weights[i].to_bits();
            idx += 1;
        }

        self.buffer[0] = MAGIC_VERSION;
        self.buffer[1] = VALID_MARKER;
    }

    /// Восстанавливает веса из буфера
    pub fn load_weights(&self) -> Option<([[f32; 7]; 8], [f32; 8])> {
        const MAGIC_VERSION: u32 = 0x4E_4F_53_01;
        const VALID_MARKER: u32 = 0x5641_4C49;

        if self.buffer[0] != MAGIC_VERSION || self.buffer[1] != VALID_MARKER {
            return None;
        }

        let mut weights = [[0.0; 7]; 8];
        let mut output_weights = [0.0; 8];
        let mut idx = 2;
        
        // Загружаем входные веса
        for i in 0..8 {
            for j in 0..7 {
                weights[i][j] = f32::from_bits(self.buffer[idx]);
                idx += 1;
            }
        }
        
        // Загружаем выходные веса
        for weight in &mut output_weights {
            *weight = f32::from_bits(self.buffer[idx]);
            idx += 1;
        }

        Some((weights, output_weights))
    }

    /// Очищает хранилище
    pub fn clear(&mut self) {
        for word in &mut self.buffer {
            *word = 0;
        }
    }
}
