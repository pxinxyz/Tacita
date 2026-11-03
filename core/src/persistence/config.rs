use super::{name, valid_name, Reader, StateError, MAX_RECORDS, MAX_TEXT};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
const MAGIC: &[u8; 8] = b"RGCF\x01\0\0\0";
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Integer(i64),
    Number(f64),
    Color([u8; 4]),
    Text(String),
}
impl Value {
    pub fn kind(&self) -> u32 {
        match self {
            Self::Bool(_) => 0,
            Self::Integer(_) => 1,
            Self::Number(_) => 2,
            Self::Color(_) => 3,
            Self::Text(_) => 4,
        }
    }
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Number(v) => v.is_finite(),
            Self::Text(v) => v.len() <= MAX_TEXT,
            _ => true,
        }
    }
    pub fn payload(&self) -> Vec<u8> {
        match self {
            Self::Bool(v) => alloc::vec![*v as u8],
            Self::Integer(v) => v.to_le_bytes().to_vec(),
            Self::Number(v) => v.to_le_bytes().to_vec(),
            Self::Color(v) => v.to_vec(),
            Self::Text(v) => v.as_bytes().to_vec(),
        }
    }
    pub fn from_payload(kind: u32, data: &[u8]) -> Result<Self, StateError> {
        let value = match kind {
            0 if data.len() == 1 && data[0] <= 1 => Self::Bool(data[0] != 0),
            1 if data.len() == 8 => Self::Integer(i64::from_le_bytes(data.try_into().unwrap())),
            2 if data.len() == 8 => Self::Number(f64::from_le_bytes(data.try_into().unwrap())),
            3 if data.len() == 4 => Self::Color(data.try_into().unwrap()),
            4 if data.len() <= MAX_TEXT => Self::Text(String::from(
                core::str::from_utf8(data).map_err(|_| StateError::Malformed)?,
            )),
            _ => return Err(StateError::InvalidValue),
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(StateError::InvalidValue)
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    values: BTreeMap<String, Value>,
}
impl Config {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, key: &str, default: Value) -> Result<(), StateError> {
        if !valid_name(key) || !default.is_valid() {
            return Err(StateError::InvalidValue);
        }
        if self.values.contains_key(key) {
            return Err(StateError::Schema);
        }
        if self.values.len() >= MAX_RECORDS {
            return Err(StateError::Limit);
        }
        self.values.insert(String::from(key), default);
        Ok(())
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }
    pub fn set(&mut self, key: &str, value: Value) -> Result<(), StateError> {
        let old = self.values.get_mut(key).ok_or(StateError::Schema)?;
        if old.kind() != value.kind() || !value.is_valid() {
            return Err(StateError::InvalidValue);
        }
        *old = value;
        Ok(())
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&(self.values.len() as u32).to_le_bytes());
        for (key, value) in &self.values {
            name(&mut out, key);
            out.push(value.kind() as u8);
            let data = value.payload();
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&data);
        }
        out
    }
    pub fn decode(data: &[u8]) -> Result<Self, StateError> {
        let mut reader = Reader::new(data, MAGIC)?;
        let count = reader.count()?;
        let mut config = Self::new();
        for _ in 0..count {
            let key = reader.name()?;
            let kind = reader.take(1)?[0];
            let len = reader.u32()? as usize;
            if len > MAX_TEXT {
                return Err(StateError::Limit);
            }
            let value = Value::from_payload(kind as u32, reader.take(len)?)?;
            config.register(&key, value)?;
        }
        reader.finish()?;
        Ok(config)
    }
    pub fn load(&mut self, data: &[u8]) -> Result<(), StateError> {
        let replacement = Self::decode(data)?;
        if self.values.len() != replacement.values.len()
            || self
                .values
                .iter()
                .any(|(k, v)| replacement.get(k).is_none_or(|new| v.kind() != new.kind()))
        {
            return Err(StateError::Schema);
        }
        *self = replacement;
        Ok(())
    }
}
