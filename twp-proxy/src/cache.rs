// Content-addressed image cache. The cache key is the low 23 bits of a
// blake3 digest of the payload bytes — this same value is reused as the
// Kitty Graphics image ID (the foreground-color encoding has 24 bits, but
// the top bit is reserved as a placeholder marker, so we keep 23 bits).
//
// On a hit we skip the PNG transmission and only re-emit the placeholder
// cells; the terminal already has the bitmap stored under that ID.

use std::collections::HashSet;

const ID_MASK: u32 = 0x7F_FFFF;

pub fn image_id_for(payload: &[u8]) -> u32 {
    let hash = blake3::hash(payload);
    let bytes = hash.as_bytes();
    let low = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    low & ID_MASK
}

pub struct Cache {
    transmitted: HashSet<u32>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            transmitted: HashSet::new(),
        }
    }

    /// Returns true on first insertion (i.e. the caller must transmit the
    /// PNG); returns false on subsequent calls with the same id.
    pub fn mark_transmitted(&mut self, id: u32) -> bool {
        self.transmitted.insert(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_id_is_23_bits() {
        let id = image_id_for(b"foo");
        assert_eq!(id & !ID_MASK, 0);
    }

    #[test]
    fn image_id_is_deterministic() {
        assert_eq!(image_id_for(b"foo"), image_id_for(b"foo"));
        assert_ne!(image_id_for(b"foo"), image_id_for(b"bar"));
    }

    #[test]
    fn cache_marks_first_insertion() {
        let mut c = Cache::new();
        assert!(c.mark_transmitted(42));
        assert!(!c.mark_transmitted(42));
        assert!(c.mark_transmitted(43));
    }
}
