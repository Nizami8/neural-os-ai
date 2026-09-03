//! Persistent storage for neural network weights.
//!
//! The live buffer lives in the `.persist` linker section (NOLOAD): it is not
//! part of BSS and is therefore not zeroed on boot. A magic header decides
//! whether a previous run left valid weights. On real hardware this section can
//! be backed by flash/EEPROM; under QEMU it survives soft resets that preserve
//! RAM contents.

use core::ptr::addr_of_mut;

const MAGIC: u32 = 0x4E4F_5331; // "NOS1"
const VERSION: u32 = 1;
const SLOTS: usize = 72; // 8*7 hidden + 8 output + 8 spare

#[repr(C)]
struct PersistBlob {
    magic: u32,
    version: u32,
    count: u32,
    checksum: u32,
    words: [u32; SLOTS],
}

impl PersistBlob {
    const fn empty() -> Self {
        PersistBlob {
            magic: 0,
            version: 0,
            count: 0,
            checksum: 0,
            words: [0; SLOTS],
        }
    }

    fn checksum_words(&self) -> u32 {
        let mut c = self.version ^ self.count;
        for w in self.words.iter() {
            c = c.wrapping_add(*w).wrapping_mul(0x0100_0193);
        }
        c
    }

    fn is_valid(&self) -> bool {
        self.magic == MAGIC && self.version == VERSION && self.checksum == self.checksum_words()
    }
}

#[link_section = ".persist"]
#[used]
static mut PERSIST: PersistBlob = PersistBlob::empty();

pub struct PersistentStorage {
    /// Working copy mirrored from / into the `.persist` section.
    pub buffer: [u32; SLOTS],
    pub valid: bool,
}

impl PersistentStorage {
    pub fn new() -> Self {
        // SAFETY: single-hart boot; no concurrent access yet.
        let persist = unsafe { &*addr_of_mut!(PERSIST) };
        if persist.is_valid() {
            PersistentStorage {
                buffer: persist.words,
                valid: true,
            }
        } else {
            PersistentStorage {
                buffer: [0; SLOTS],
                valid: false,
            }
        }
    }

    pub fn save_weights(&mut self, weights: &[[f32; 7]; 8], output_weights: &[f32; 8]) {
        let mut idx = 0;
        for i in 0..8 {
            for j in 0..7 {
                if idx < SLOTS {
                    self.buffer[idx] = weights[i][j].to_bits();
                    idx += 1;
                }
            }
        }
        for i in 0..8 {
            if idx < SLOTS {
                self.buffer[idx] = output_weights[i].to_bits();
                idx += 1;
            }
        }
        self.valid = true;
        self.commit();
    }

    /// Commit the working buffer into the durable `.persist` section.
    pub fn commit(&self) {
        // SAFETY: exclusive during timer-masked trap / boot.
        let persist = unsafe { &mut *addr_of_mut!(PERSIST) };
        persist.magic = MAGIC;
        persist.version = VERSION;
        persist.count = persist.count.wrapping_add(1);
        persist.words = self.buffer;
        persist.checksum = persist.checksum_words();
    }

    pub fn load_weights(&self) -> Option<([[f32; 7]; 8], [f32; 8])> {
        if !self.valid {
            return None;
        }
        let mut hidden = [[0.0f32; 7]; 8];
        let mut output = [0.0f32; 8];
        let mut idx = 0;
        for i in 0..8 {
            for j in 0..7 {
                if idx < SLOTS {
                    hidden[i][j] = f32::from_bits(self.buffer[idx]);
                    idx += 1;
                }
            }
        }
        for i in 0..8 {
            if idx < SLOTS {
                output[i] = f32::from_bits(self.buffer[idx]);
                idx += 1;
            }
        }
        Some((hidden, output))
    }

    pub fn clear(&mut self) {
        self.buffer = [0; SLOTS];
        self.valid = false;
        let persist = unsafe { &mut *addr_of_mut!(PERSIST) };
        *persist = PersistBlob::empty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::neural::fabs;

    #[test]
    fn save_load_roundtrip() {
        let mut s = PersistentStorage {
            buffer: [0; SLOTS],
            valid: false,
        };

        let mut w = [[0.0f32; 7]; 8];
        for i in 0..8 {
            for j in 0..7 {
                w[i][j] = (i * 7 + j) as f32 * 0.01;
            }
        }
        let out = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8f32];

        s.save_weights(&w, &out);
        let (loaded_h, loaded_o) = s.load_weights().expect("load_weights returned None");

        for i in 0..8 {
            for j in 0..7 {
                assert!(fabs(loaded_h[i][j] - w[i][j]) < 1e-6);
            }
            assert!(fabs(loaded_o[i] - out[i]) < 1e-6);
        }
    }

    #[test]
    fn clear_invalidates() {
        let mut s = PersistentStorage {
            buffer: [0; SLOTS],
            valid: false,
        };
        let w = [[1.0f32; 7]; 8];
        let out = [1.0f32; 8];
        s.save_weights(&w, &out);
        assert!(s.load_weights().is_some());
        s.clear();
        assert!(s.load_weights().is_none());
    }
}
