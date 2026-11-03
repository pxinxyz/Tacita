use crate::{
    input::{InputState, Key, KeyEvent},
    interaction::Navigation,
};
use alloc::vec::Vec;
const KEYS: [Key; 64] = [
    Key::Backspace,
    Key::Delete,
    Key::Left,
    Key::Right,
    Key::Home,
    Key::End,
    Key::Enter,
    Key::Escape,
    Key::Insert,
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::Space,
    Key::Tab,
    Key::Shift,
    Key::Ctrl,
    Key::Alt,
    Key::Up,
    Key::Down,
];
pub const MAX_RECORDING_FRAMES: usize = 4096;
pub const MAX_RECORDING_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayError {
    InvalidInput,
    LimitExceeded,
}
#[derive(Clone, Debug)]
pub struct RecordedFrame {
    pub dt: f32,
    mouse: [f32; 2],
    down: [bool; 5],
    wheel: f32,
    viewport: [f32; 2],
    keys: Vec<KeyEvent>,
    navigation: Vec<Navigation>,
}
impl RecordedFrame {
    pub fn input(&self) -> InputState<'_> {
        InputState {
            mouse_pos: self.mouse,
            mouse_down: self.down,
            mouse_wheel: self.wheel,
            viewport_size: self.viewport,
            keys: &self.keys,
        }
    }
    pub fn navigation(&self) -> &[Navigation] {
        &self.navigation
    }
}
#[derive(Default, Clone, Debug)]
pub struct InputRecording {
    frames: Vec<RecordedFrame>,
    bytes: usize,
}
impl InputRecording {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.frames.len()
    }
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
    pub fn clear(&mut self) {
        self.frames.clear();
        self.bytes = 0;
    }
    pub fn frame(&self, index: usize) -> Option<&RecordedFrame> {
        self.frames.get(index)
    }
    pub fn push(
        &mut self,
        input: InputState<'_>,
        dt: f32,
        navigation: &[Navigation],
    ) -> Result<(), ReplayError> {
        if !dt.is_finite()
            || dt < 0.
            || !input
                .mouse_pos
                .iter()
                .chain(&input.viewport_size)
                .chain(core::iter::once(&input.mouse_wheel))
                .all(|v| v.is_finite())
            || input.viewport_size.iter().any(|v| *v < 0.)
        {
            return Err(ReplayError::InvalidInput);
        }
        let bytes = 31 + input.keys.len() * 5 + navigation.len();
        if input.keys.len() > 1024
            || navigation.len() > 64
            || self.len() >= MAX_RECORDING_FRAMES
            || self.bytes + bytes > MAX_RECORDING_BYTES - 12
        {
            return Err(ReplayError::LimitExceeded);
        }
        self.frames.push(RecordedFrame {
            dt,
            mouse: input.mouse_pos,
            down: input.mouse_down,
            wheel: input.mouse_wheel,
            viewport: input.viewport_size,
            keys: input.keys.to_vec(),
            navigation: navigation.to_vec(),
        });
        self.bytes += bytes;
        Ok(())
    }
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.clear();
        out.extend_from_slice(b"RGUIREP1");
        out.extend_from_slice(&(self.len() as u32).to_le_bytes());
        for frame in &self.frames {
            for v in [
                frame.dt,
                frame.mouse[0],
                frame.mouse[1],
                frame.wheel,
                frame.viewport[0],
                frame.viewport[1],
            ] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.push(
                frame
                    .down
                    .iter()
                    .enumerate()
                    .fold(0, |mask, (i, &on)| mask | (u8::from(on) << i)),
            );
            out.extend_from_slice(&(frame.keys.len() as u16).to_le_bytes());
            out.extend_from_slice(&(frame.navigation.len() as u16).to_le_bytes());
            for event in &frame.keys {
                let (tag, value) = match event {
                    KeyEvent::Press(k) => (0, *k as u32),
                    KeyEvent::Release(k) => (1, *k as u32),
                    KeyEvent::Char(c) => (2, *c as u32),
                };
                out.push(tag);
                out.extend_from_slice(&value.to_le_bytes());
            }
            for event in &frame.navigation {
                out.push(*event as u8);
            }
        }
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ReplayError> {
        if bytes.len() > MAX_RECORDING_BYTES {
            return Err(ReplayError::LimitExceeded);
        }
        let mut r = Reader { bytes, offset: 0 };
        if r.take(8)? != b"RGUIREP1" {
            return Err(ReplayError::InvalidInput);
        }
        let count = r.u32()? as usize;
        if count > MAX_RECORDING_FRAMES {
            return Err(ReplayError::LimitExceeded);
        }
        let mut recording = Self::new();
        for _ in 0..count {
            let dt = r.f32()?;
            let mouse_pos = [r.f32()?, r.f32()?];
            let mouse_wheel = r.f32()?;
            let viewport_size = [r.f32()?, r.f32()?];
            let mask = r.u8()?;
            if mask & !31 != 0 {
                return Err(ReplayError::InvalidInput);
            }
            let key_count = r.u16()? as usize;
            let nav_count = r.u16()? as usize;
            if key_count > 1024 || nav_count > 64 {
                return Err(ReplayError::LimitExceeded);
            }
            let mut keys = Vec::with_capacity(key_count);
            for _ in 0..key_count {
                let tag = r.u8()?;
                let value = r.u32()?;
                keys.push(match tag {
                    0 => {
                        KeyEvent::Press(*KEYS.get(value as usize).ok_or(ReplayError::InvalidInput)?)
                    }
                    1 => KeyEvent::Release(
                        *KEYS.get(value as usize).ok_or(ReplayError::InvalidInput)?,
                    ),
                    2 => KeyEvent::Char(char::from_u32(value).ok_or(ReplayError::InvalidInput)?),
                    _ => return Err(ReplayError::InvalidInput),
                });
            }
            let mut navigation = Vec::with_capacity(nav_count);
            for _ in 0..nav_count {
                navigation.push(match r.u8()? {
                    0 => Navigation::Next,
                    1 => Navigation::Previous,
                    2 => Navigation::Left,
                    3 => Navigation::Right,
                    4 => Navigation::Up,
                    5 => Navigation::Down,
                    6 => Navigation::Activate,
                    7 => Navigation::Cancel,
                    _ => return Err(ReplayError::InvalidInput),
                });
            }
            recording.push(
                InputState {
                    mouse_pos,
                    mouse_down: core::array::from_fn(|i| mask & (1 << i) != 0),
                    mouse_wheel,
                    viewport_size,
                    keys: &keys,
                },
                dt,
                &navigation,
            )?;
        }
        if r.offset != bytes.len() {
            return Err(ReplayError::InvalidInput);
        }
        Ok(recording)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], ReplayError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(ReplayError::InvalidInput)?;
        let data = self
            .bytes
            .get(self.offset..end)
            .ok_or(ReplayError::InvalidInput)?;
        self.offset = end;
        Ok(data)
    }
    fn u8(&mut self) -> Result<u8, ReplayError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ReplayError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, ReplayError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Result<f32, ReplayError> {
        Ok(f32::from_bits(self.u32()?))
    }
}
#[derive(Default, Clone, Copy, Debug)]
pub struct ReplayCursor {
    position: usize,
}
impl ReplayCursor {
    pub fn position(self) -> usize {
        self.position
    }
    pub fn restart(&mut self) {
        self.position = 0;
    }
    pub fn seek(&mut self, recording: &InputRecording, index: usize) -> Result<(), ReplayError> {
        if index > recording.len() {
            return Err(ReplayError::InvalidInput);
        }
        self.position = index;
        Ok(())
    }
    pub fn next<'a>(&mut self, recording: &'a InputRecording) -> Option<&'a RecordedFrame> {
        let frame = recording.frame(self.position)?;
        self.position += 1;
        Some(frame)
    }
}
impl crate::context::Context {
    pub fn replay_frame(&mut self, frame: &RecordedFrame) {
        self.set_input(frame.input());
        for &event in frame.navigation() {
            self.navigate(event);
        }
        self.new_frame(frame.dt);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_unicode_navigation_and_cursor() {
        let mut recording = InputRecording::new();
        let keys = [
            KeyEvent::Press(Key::Ctrl),
            KeyEvent::Char('界'),
            KeyEvent::Release(Key::Ctrl),
        ];
        recording
            .push(
                InputState {
                    mouse_pos: [1.25, 2.5],
                    keys: &keys,
                    ..Default::default()
                },
                0.125,
                &[Navigation::Next],
            )
            .unwrap();
        let mut bytes = Vec::new();
        recording.encode(&mut bytes);
        let decoded = InputRecording::decode(&bytes).unwrap();
        let mut cursor = ReplayCursor::default();
        let frame = cursor.next(&decoded).unwrap();
        assert_eq!(frame.input().keys, keys);
        assert_eq!(frame.dt, 0.125);
        assert_eq!(frame.navigation(), [Navigation::Next]);
        assert!(cursor.next(&decoded).is_none());
        cursor.restart();
        assert!(cursor.next(&decoded).is_some());
        for len in 0..bytes.len() {
            assert!(InputRecording::decode(&bytes[..len]).is_err());
        }
        bytes.push(0);
        assert!(InputRecording::decode(&bytes).is_err());
    }
    #[test]
    fn invalid_frame_does_not_change_recording() {
        let mut recording = InputRecording::new();
        assert!(recording
            .push(InputState::default(), f32::NAN, &[])
            .is_err());
        assert!(recording.is_empty());
    }
}
