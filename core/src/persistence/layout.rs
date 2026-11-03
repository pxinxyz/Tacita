use super::{name, scalar, valid_name, Reader, StateError, MAX_RECORDS};
use crate::{
    context::Context,
    panels::{DockState, DockZone, SplitAxis},
};
use alloc::{string::String, vec::Vec};
const MAGIC: &[u8; 8] = b"RGLY\x01\0\0\0";
#[derive(Clone, Debug, PartialEq)]
pub struct WindowLayout {
    pub title: String,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub scroll: f32,
    pub open: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanelState {
    Rect([f32; 4]),
    Size([f32; 2]),
    Split { axis: SplitAxis, ratio: f32 },
    Dock(DockState),
    Scroll(f32),
}
impl PanelState {
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::Rect(r) => finite(&r) && r[2] > r[0] && r[3] > r[1],
            Self::Size(s) => finite(&s) && s[0] > 0. && s[1] > 0.,
            Self::Split { ratio, .. } => ratio.is_finite() && (0.0..=1.0).contains(&ratio),
            Self::Dock(s) => finite(&s.float_pos),
            Self::Scroll(v) => finite(&[v]) && v >= 0.,
        }
    }
}
fn finite(values: &[f32]) -> bool {
    values
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
}
#[derive(Clone, Debug, PartialEq)]
pub struct PanelLayout {
    pub name: String,
    pub state: PanelState,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutSnapshot {
    pub windows: Vec<WindowLayout>,
    pub panels: Vec<PanelLayout>,
}
impl LayoutSnapshot {
    pub fn set_panel(&mut self, key: &str, state: PanelState) -> Result<(), StateError> {
        if !valid_name(key) || !state.is_valid() {
            return Err(StateError::InvalidValue);
        }
        if let Some(old) = self.panels.iter_mut().find(|p| p.name == key) {
            old.state = state;
            return Ok(());
        }
        if self.panels.len() >= MAX_RECORDS {
            return Err(StateError::Limit);
        }
        self.panels.push(PanelLayout {
            name: String::from(key),
            state,
        });
        Ok(())
    }
    pub fn panel(&self, key: &str) -> Option<PanelState> {
        self.panels.iter().find(|p| p.name == key).map(|p| p.state)
    }
    pub fn is_valid(&self) -> bool {
        self.windows.len() <= MAX_RECORDS
            && self.panels.len() <= MAX_RECORDS
            && self.windows.iter().enumerate().all(|(i, w)| {
                valid_name(&w.title)
                    && finite(&w.pos)
                    && finite(&w.size)
                    && w.size[0] >= crate::context::MIN_WINDOW_SIZE[0]
                    && w.size[1] >= crate::context::MIN_WINDOW_SIZE[1]
                    && finite(&[w.scroll])
                    && w.scroll >= 0.
                    && !self.windows[..i].iter().any(|p| p.title == w.title)
            })
            && self.panels.iter().enumerate().all(|(i, p)| {
                valid_name(&p.name)
                    && p.state.is_valid()
                    && !self.panels[..i].iter().any(|q| q.name == p.name)
            })
    }
    pub fn encode(&self) -> Result<Vec<u8>, StateError> {
        if !self.is_valid() {
            return Err(StateError::InvalidValue);
        }
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&(self.windows.len() as u32).to_le_bytes());
        for w in &self.windows {
            name(&mut out, &w.title);
            for v in w.pos.into_iter().chain(w.size).chain([w.scroll]) {
                scalar(&mut out, v);
            }
            out.push(w.open as u8);
        }
        out.extend_from_slice(&(self.panels.len() as u32).to_le_bytes());
        for p in &self.panels {
            name(&mut out, &p.name);
            let (kind, mode, values) = match p.state {
                PanelState::Rect(r) => (0, 0, r),
                PanelState::Size(s) => (1, 0, [s[0], s[1], 0., 0.]),
                PanelState::Split { axis, ratio } => (
                    2,
                    if axis == SplitAxis::Horizontal { 0 } else { 1 },
                    [ratio, 0., 0., 0.],
                ),
                PanelState::Dock(s) => (3, s.zone as u32, [s.float_pos[0], s.float_pos[1], 0., 0.]),
                PanelState::Scroll(v) => (4, 0, [v, 0., 0., 0.]),
            };
            out.extend_from_slice(&(kind as u32).to_le_bytes());
            out.extend_from_slice(&mode.to_le_bytes());
            for v in values {
                scalar(&mut out, v);
            }
        }
        Ok(out)
    }
    pub fn decode(data: &[u8]) -> Result<Self, StateError> {
        let mut r = Reader::new(data, MAGIC)?;
        let count = r.count()?;
        let mut s = Self::default();
        for _ in 0..count {
            let title = r.name()?;
            let pos = [r.f32()?, r.f32()?];
            let size = [r.f32()?, r.f32()?];
            let scroll = r.f32()?;
            let open = match r.take(1)?[0] {
                0 => false,
                1 => true,
                _ => return Err(StateError::InvalidValue),
            };
            s.windows.push(WindowLayout {
                title,
                pos,
                size,
                scroll,
                open,
            });
        }
        let count = r.count()?;
        for _ in 0..count {
            let key = r.name()?;
            let kind = r.u32()?;
            let mode = r.u32()?;
            let v = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
            let state = match (kind, mode) {
                (0, 0) => PanelState::Rect(v),
                (1, 0) => PanelState::Size([v[0], v[1]]),
                (2, 0 | 1) => PanelState::Split {
                    axis: if mode == 0 {
                        SplitAxis::Horizontal
                    } else {
                        SplitAxis::Vertical
                    },
                    ratio: v[0],
                },
                (3, 0..=2) => PanelState::Dock(DockState {
                    zone: match mode {
                        0 => DockZone::Left,
                        1 => DockZone::Right,
                        _ => DockZone::Floating,
                    },
                    float_pos: [v[0], v[1]],
                }),
                (4, 0) => PanelState::Scroll(v[0]),
                _ => return Err(StateError::InvalidValue),
            };
            let used = match kind {
                0 => 4,
                1 | 3 => 2,
                _ => 1,
            };
            if v[used..].iter().any(|v| *v != 0.) {
                return Err(StateError::InvalidValue);
            }
            if s.panel(&key).is_some() {
                return Err(StateError::Schema);
            }
            s.set_panel(&key, state)?;
        }
        r.finish()?;
        if s.is_valid() {
            Ok(s)
        } else {
            Err(StateError::InvalidValue)
        }
    }
}
impl Context {
    pub fn save_layout(&self) -> LayoutSnapshot {
        assert!(
            !self.frame_active,
            "layout snapshots require an idle context"
        );
        LayoutSnapshot {
            windows: self
                .windows
                .iter()
                .map(|w| WindowLayout {
                    title: w.title.clone(),
                    pos: w.pos,
                    size: w.size,
                    scroll: w.scroll_offset,
                    open: w.open,
                })
                .collect(),
            panels: Vec::new(),
        }
    }
    pub fn load_layout(&mut self, s: &LayoutSnapshot) -> Result<(), StateError> {
        assert!(!self.frame_active, "layout load requires an idle context");
        if !s.is_valid() {
            return Err(StateError::InvalidValue);
        }
        while self.popup_depth() > 0 {
            self.dismiss_popup();
        }
        self.clear_active();
        self.focused = None;
        self.focused_in_popup = false;
        self.focused_in_modal = false;
        for (position, w) in s.windows.iter().enumerate() {
            self.set_window_rect(&w.title, w.pos, w.size);
            let index = self
                .windows
                .iter()
                .position(|window| window.title == w.title)
                .unwrap();
            self.windows[index].open = w.open;
            self.windows[index].scroll_offset = w.scroll;
            self.windows.swap(position, index);
        }
        Ok(())
    }
}
