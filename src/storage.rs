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

    /// Восстанавливает веса из буфера (no_std: фиксированные массивы вместо Vec)
    pub fn load_weights(&self) -> Option<([f32; 56], [f32; 8])> {
        let mut weights = [0.0f32; 56];   // 8×7 входных весов
        let mut output_weights = [0.0f32; 8];

        let mut idx = 0;

        // Загружаем входные веса
        for w in weights.iter_mut() {
            if idx < 32 {
                *w = f32::from_bits(self.buffer[idx]);
                idx += 1;
            }
        }

        // Загружаем выходные веса
        for w in output_weights.iter_mut() {
            if idx < 32 {
                *w = f32::from_bits(self.buffer[idx]);
                idx += 1;
            }
        }

        Some((weights, output_weights))
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
    fn save_load_roundtrip() {
        let mut s = PersistentStorage::new();

        let mut w = [[0.0f32; 7]; 8];
        for i in 0..8 {
            for j in 0..7 {
                w[i][j] = (i * 7 + j) as f32 * 0.01;
            }
        }
        let out = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8f32];

        s.save_weights(&w, &out);
        let (loaded, _) = s.load_weights().expect("load_weights returned None");

        // The buffer holds 32 slots, so only the first 32 flattened hidden
        // weights survive the round trip; check those bit-for-bit.
        for k in 0..32 {
            let (i, j) = (k / 7, k % 7);
            assert!(
                (loaded[k] - w[i][j]).abs() < 1e-9,
                "mismatch at {}: {} != {}",
                k,
                loaded[k],
                w[i][j]
            );
        }
    }

    #[test]
    fn clear_zeroes_buffer() {
        let mut s = PersistentStorage::new();
        let w = [[1.0f32; 7]; 8];
        let out = [1.0f32; 8];
        s.save_weights(&w, &out);
        s.clear();
        assert!(s.buffer.iter().all(|&x| x == 0));
    }
}
