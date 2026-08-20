//! Capability table: object type + rights + generation, never a raw PID.

#![allow(dead_code)]

use crate::config::MAX_CAPS;

pub type CapIndex = usize;

pub const RIGHT_SEND: u32 = 1 << 0;
pub const RIGHT_RECV: u32 = 1 << 1;
pub const RIGHT_GRANT: u32 = 1 << 2;
pub const RIGHT_MAP: u32 = 1 << 3;
pub const RIGHT_DELETE: u32 = 1 << 4;

pub const RIGHT_IPC: u32 = RIGHT_SEND | RIGHT_RECV;
pub const RIGHT_ALL: u32 = RIGHT_SEND | RIGHT_RECV | RIGHT_GRANT | RIGHT_MAP | RIGHT_DELETE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectType {
    None,
    Endpoint,
    Process,
    Thread,
}

impl ObjectType {
    pub const fn as_u8(self) -> u8 {
        match self {
            ObjectType::None => 0,
            ObjectType::Endpoint => 1,
            ObjectType::Process => 2,
            ObjectType::Thread => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capability {
    pub object_type: ObjectType,
    pub object_id: u16,
    pub generation: u32,
    pub rights: u32,
    pub owner: u16,
    pub flags: u32,
    pub expiry_ticks: Option<u64>,
}

impl Capability {
    pub const fn empty() -> Self {
        Self {
            object_type: ObjectType::None,
            object_id: 0,
            generation: 0,
            rights: 0,
            owner: 0,
            flags: 0,
            expiry_ticks: None,
        }
    }

    pub fn endpoint(object_id: u16, generation: u32, rights: u32, owner: u16) -> Self {
        Self {
            object_type: ObjectType::Endpoint,
            object_id,
            generation,
            rights,
            owner,
            flags: 0,
            expiry_ticks: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.object_type == ObjectType::None
    }

    pub fn has_right(&self, right: u32) -> bool {
        self.rights & right == right
    }

    pub fn is_expired(&self, now: u64) -> bool {
        match self.expiry_ticks {
            Some(exp) => now >= exp,
            None => false,
        }
    }

    pub fn with_expiry(mut self, ticks: u64) -> Self {
        self.expiry_ticks = Some(ticks);
        self
    }

    pub fn restricted(&self, rights: u32) -> Self {
        let mut copy = *self;
        copy.rights &= rights;
        copy
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapError {
    EmptySlot,
    WrongType,
    Denied,
    StaleGeneration,
    Expired,
    TableFull,
    BadIndex,
}

#[derive(Clone, Copy)]
pub struct CapabilityTable {
    slots: [Capability; MAX_CAPS],
}

impl CapabilityTable {
    pub const fn new() -> Self {
        Self {
            slots: [Capability::empty(); MAX_CAPS],
        }
    }

    pub fn get(&self, index: CapIndex) -> Result<&Capability, CapError> {
        if index >= MAX_CAPS {
            return Err(CapError::BadIndex);
        }
        let cap = &self.slots[index];
        if cap.is_empty() {
            Err(CapError::EmptySlot)
        } else {
            Ok(cap)
        }
    }

    pub fn get_mut(&mut self, index: CapIndex) -> Result<&mut Capability, CapError> {
        if index >= MAX_CAPS {
            return Err(CapError::BadIndex);
        }
        let cap = &mut self.slots[index];
        if cap.is_empty() {
            Err(CapError::EmptySlot)
        } else {
            Ok(cap)
        }
    }

    pub fn lookup_endpoint(
        &self,
        index: CapIndex,
        need: u32,
        generation: u32,
        now: u64,
    ) -> Result<u16, CapError> {
        let cap = self.get(index)?;
        if cap.object_type != ObjectType::Endpoint {
            return Err(CapError::WrongType);
        }
        if !cap.has_right(need) {
            return Err(CapError::Denied);
        }
        if cap.generation != generation {
            return Err(CapError::StaleGeneration);
        }
        if cap.is_expired(now) {
            return Err(CapError::Expired);
        }
        Ok(cap.object_id)
    }

    pub fn insert(&mut self, cap: Capability) -> Result<CapIndex, CapError> {
        for i in 1..MAX_CAPS {
            if self.slots[i].is_empty() {
                self.slots[i] = cap;
                return Ok(i);
            }
        }
        Err(CapError::TableFull)
    }

    pub fn insert_at(&mut self, index: CapIndex, cap: Capability) -> Result<(), CapError> {
        if index == 0 || index >= MAX_CAPS {
            return Err(CapError::BadIndex);
        }
        self.slots[index] = cap;
        Ok(())
    }

    pub fn remove(&mut self, index: CapIndex) -> Result<Capability, CapError> {
        let cap = *self.get(index)?;
        self.slots[index] = Capability::empty();
        Ok(cap)
    }

    pub fn grant(&mut self, index: CapIndex, rights: u32) -> Result<Capability, CapError> {
        let cap = self.get(index)?;
        if !cap.has_right(RIGHT_GRANT) {
            return Err(CapError::Denied);
        }
        Ok(cap.restricted(rights | (cap.rights & RIGHT_GRANT)))
    }

    pub fn slot(&self, index: CapIndex) -> Option<&Capability> {
        self.slots.get(index).filter(|c| !c.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_missing_send_right() {
        let mut table = CapabilityTable::new();
        table
            .insert(Capability::endpoint(1, 1, RIGHT_RECV, 1))
            .unwrap();
        let err = table.lookup_endpoint(1, RIGHT_SEND, 1, 0).unwrap_err();
        assert_eq!(err, CapError::Denied);
    }

    #[test]
    fn rejects_stale_generation() {
        let mut table = CapabilityTable::new();
        table
            .insert(Capability::endpoint(1, 7, RIGHT_SEND, 1))
            .unwrap();
        let err = table.lookup_endpoint(1, RIGHT_SEND, 8, 0).unwrap_err();
        assert_eq!(err, CapError::StaleGeneration);
    }

    #[test]
    fn grant_requires_grant_right() {
        let mut table = CapabilityTable::new();
        let idx = table
            .insert(Capability::endpoint(1, 1, RIGHT_SEND, 1))
            .unwrap();
        assert_eq!(table.grant(idx, RIGHT_SEND), Err(CapError::Denied));
    }
}
