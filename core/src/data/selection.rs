use super::{DataError, MAX_DATA_ROWS};
use alloc::vec::Vec;

#[derive(Default, Clone, Debug)]
pub struct Selection {
    bits: Vec<u64>,
    count: usize,
    anchor: Option<usize>,
}
impl Selection {
    pub fn new(count: usize) -> Result<Self, DataError> {
        let mut s = Self::default();
        s.resize(count)?;
        Ok(s)
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn resize(&mut self, count: usize) -> Result<(), DataError> {
        if count > MAX_DATA_ROWS {
            return Err(DataError::InvalidInput);
        }
        self.bits.resize(count.div_ceil(64), 0);
        if !count.is_multiple_of(64) {
            if let Some(last) = self.bits.last_mut() {
                *last &= (1u64 << (count % 64)) - 1;
            }
        }
        self.count = count;
        if self.anchor.is_some_and(|i| i >= count) {
            self.anchor = None;
        }
        Ok(())
    }
    pub fn is_selected(&self, index: usize) -> bool {
        index < self.count && self.bits[index / 64] & (1u64 << (index % 64)) != 0
    }
    pub fn selected_count(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }
    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.bits.iter().enumerate().flat_map(|(word, &bits)| {
            let mut bits = bits;
            core::iter::from_fn(move || {
                if bits == 0 {
                    return None;
                }
                let bit = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                Some(word * 64 + bit)
            })
        })
    }
    pub fn clear(&mut self) -> bool {
        let changed = self.bits.iter().any(|b| *b != 0);
        self.bits.fill(0);
        self.anchor = None;
        changed
    }
    pub fn select(&mut self, index: usize, ctrl: bool, shift: bool) -> bool {
        if index >= self.count {
            return false;
        }
        let anchor = if shift {
            self.anchor.unwrap_or(index)
        } else {
            index
        };
        let low = anchor.min(index);
        let high = anchor.max(index);
        let mut changed = false;
        for (word, bits) in self.bits.iter_mut().enumerate() {
            let begin = word * 64;
            let end = begin + 63;
            let mask = if low > end || high < begin {
                0
            } else {
                let a = low.saturating_sub(begin);
                let b = (high - begin).min(63);
                (u64::MAX << a) & (u64::MAX >> (63 - b))
            };
            let next = if ctrl && !shift {
                *bits ^ mask
            } else if ctrl {
                *bits | mask
            } else {
                mask
            };
            changed |= next != *bits;
            *bits = next;
        }
        if !shift || self.anchor.is_none() {
            self.anchor = Some(index);
        }
        changed
    }
}
