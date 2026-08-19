/// Persistent storage for neural network weights
/// Saves/loads learned weights across system restarts

pub const STORAGE_SLOTS: usize = 32;  // 32 x 32-bit = 128 bytes для весов

pub struct PersistentStorage {
    pub buffer: [u32; STORAGE_SLOTS],
}

impl PersistentStorage {
    pub fn new() -> Self {
        PersistentStorage {
            buffer: [0; STORAGE_SLOTS],
        }
    }

    /// Записывает значение в слот cursor и сдвигает курсор, пока есть место
    fn write_slot(&mut self, cursor: &mut usize, value: f32) {
        if *cursor < STORAGE_SLOTS {
            self.buffer[*cursor] = value.to_bits();
            *cursor += 1;
        }
    }

    /// Читает подряд идущие count значений начиная с cursor (не выходя за буфер)
    fn read_slots(&self, cursor: &mut usize, count: usize, out: &mut Vec<f32>) {
        for _ in 0..count {
            if *cursor < STORAGE_SLOTS {
                out.push(f32::from_bits(self.buffer[*cursor]));
                *cursor += 1;
            }
        }
    }

    /// Сохраняет веса нейросети в буфер (в реальном OS это была бы запись в EEPROM/Flash)
    pub fn save_weights(&mut self, weights: &[[f32; 7]; 8], output_weights: &[f32; 8]) {
        let mut cursor = 0;

        // Сохраняем входные веса (8 нейронов × 7 входов)
        for row in weights.iter() {
            for value in row.iter() {
                self.write_slot(&mut cursor, *value);
            }
        }

        // Сохраняем выходные веса (8 весов)
        for value in output_weights.iter() {
            self.write_slot(&mut cursor, *value);
        }
    }

    /// Восстанавливает веса из буфера
    pub fn load_weights(&self) -> Option<(Vec<f32>, Vec<f32>)> {
        let mut weights = Vec::new();
        let mut output_weights = Vec::new();
        let mut cursor = 0;

        self.read_slots(&mut cursor, 8 * 7, &mut weights);
        self.read_slots(&mut cursor, 8, &mut output_weights);

        if !weights.is_empty() && !output_weights.is_empty() {
            Some((weights, output_weights))
        } else {
            None
        }
    }

    /// Очищает хранилище
    pub fn clear(&mut self) {
        self.buffer = [0; STORAGE_SLOTS];
    }
}
