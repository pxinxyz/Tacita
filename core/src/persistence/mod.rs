mod config;
mod history;
mod layout;
mod theme;
use alloc::{string::String, vec::Vec};
pub use config::{Config, Value};
pub use history::History;
pub use layout::{LayoutSnapshot, PanelLayout, PanelState, WindowLayout};
pub use theme::{decode_theme, encode_theme};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateError {
    Version,
    Malformed,
    Limit,
    Schema,
    InvalidValue,
}
pub(crate) const MAX_RECORDS: usize = 256;
pub(crate) const MAX_TEXT: usize = 4096;
pub(crate) fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
}
pub(crate) struct Reader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}
impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], magic: &[u8; 8]) -> Result<Self, StateError> {
        if data.len() > 2 * 1024 * 1024 {
            return Err(StateError::Limit);
        }
        if data.get(..8) != Some(magic) {
            return Err(StateError::Version);
        }
        Ok(Self { data, pos: 8 })
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], StateError> {
        let end = self.pos.checked_add(n).ok_or(StateError::Limit)?;
        let bytes = self.data.get(self.pos..end).ok_or(StateError::Malformed)?;
        self.pos = end;
        Ok(bytes)
    }
    pub fn u32(&mut self) -> Result<u32, StateError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> Result<f32, StateError> {
        Ok(f32::from_bits(self.u32()?))
    }
    pub fn name(&mut self) -> Result<String, StateError> {
        let n = u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as usize;
        let name = core::str::from_utf8(self.take(n)?).map_err(|_| StateError::Malformed)?;
        if !valid_name(name) {
            return Err(StateError::InvalidValue);
        }
        Ok(String::from(name))
    }
    pub fn count(&mut self) -> Result<usize, StateError> {
        let n = self.u32()? as usize;
        if n > MAX_RECORDS {
            Err(StateError::Limit)
        } else {
            Ok(n)
        }
    }
    pub fn finish(self) -> Result<(), StateError> {
        if self.pos == self.data.len() {
            Ok(())
        } else {
            Err(StateError::Malformed)
        }
    }
}
pub(crate) fn name(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}
pub(crate) fn scalar(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}
#[cfg(test)]
mod tests;
