/// Persistent storage for neural network weights.
/// Saves/loads learned weights across system restarts.

pub struct PersistentStorage {
    pub buffer: [u32; 32],
}

impl PersistentStorage {
    pub fn new() -> Self {
        PersistentStorage { buffer: [0; 32] }
    }

    pub fn save_weights(&mut self, weights: &[[f32; 7]; 8], output_weights: &[f32; 8]) {
        let mut idx = 0;

        for row in weights.iter() {
            for value in row.iter() {
                if idx < 32 {
                    self.buffer[idx] = value.to_bits();
                    idx += 1;
                }
            }
        }

        for value in output_weights.iter() {
            if idx < 32 {
                self.buffer[idx] = value.to_bits();
                idx += 1;
            }
        }
    }

    pub fn load_weights(&self) -> Option<([[f32; 7]; 8], [f32; 8])> {
        if self.buffer.iter().all(|&w| w == 0) {
            return None;
        }

        let mut weights = [[0.0f32; 7]; 8];
        let mut output = [0.0f32; 8];
        let mut idx = 0;

        for row in weights.iter_mut() {
            for value in row.iter_mut() {
                if idx < 32 {
                    *value = f32::from_bits(self.buffer[idx]);
                    idx += 1;
                }
            }
        }

        for value in output.iter_mut() {
            if idx < 32 {
                *value = f32::from_bits(self.buffer[idx]);
                idx += 1;
            }
        }

        Some((weights, output))
    }

    pub fn clear(&mut self) {
        self.buffer = [0; 32];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_nonzero_weights() {
        let mut store = PersistentStorage::new();
        let mut hidden = [[0.0f32; 7]; 8];
        hidden[0][0] = 0.75;
        let output = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        store.save_weights(&hidden, &output);
        let (h, o) = store.load_weights().unwrap();
        assert!((h[0][0] - 0.75).abs() < 1e-6);
        assert!((o[0] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn empty_buffer_loads_nothing() {
        let store = PersistentStorage::new();
        assert!(store.load_weights().is_none());
    }
}
