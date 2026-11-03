use crate::{context::Context, id::Id};
use alloc::vec::Vec;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DuplicateId {
    pub id: Id,
    pub first: [f32; 4],
    pub second: [f32; 4],
}
#[derive(Default)]
pub(crate) struct DuplicateIds {
    pub enabled: bool,
    pub seen: Vec<(Id, [f32; 4])>,
    pub duplicates: Vec<DuplicateId>,
    pub overflow: bool,
}
impl DuplicateIds {
    pub(crate) fn reset(&mut self) {
        self.seen.clear();
        self.duplicates.clear();
        self.overflow = false;
    }
    pub(crate) fn register(&mut self, id: Id, rect: [f32; 4]) {
        if !self.enabled {
            return;
        }
        match self.seen.binary_search_by_key(&id, |e| e.0) {
            Ok(i) => {
                if self.duplicates.len() < 8192 {
                    self.duplicates.push(DuplicateId {
                        id,
                        first: self.seen[i].1,
                        second: rect,
                    });
                } else {
                    self.overflow = true;
                }
            }
            Err(i) => {
                if self.seen.len() < 8192 {
                    self.seen.insert(i, (id, rect));
                } else {
                    self.overflow = true;
                }
            }
        }
    }
}
impl Context {
    pub fn set_duplicate_id_diagnostics(&mut self, on: bool) {
        assert!(!self.frame_active, "diagnostics change between frames");
        self.duplicate_ids.enabled = on;
        self.duplicate_ids.reset();
    }
    pub fn duplicate_id_diagnostics(&self) -> bool {
        self.duplicate_ids.enabled
    }
    pub fn duplicate_ids(&self) -> &[DuplicateId] {
        &self.duplicate_ids.duplicates
    }
    pub fn duplicate_ids_overflowed(&self) -> bool {
        self.duplicate_ids.overflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScrollFlags;
    #[test]
    fn only_repeated_declarations_in_the_same_scope_are_duplicates() {
        let mut c = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
        c.set_duplicate_id_diagnostics(true);
        c.new_frame(0.016);
        c.ui().window("root", ScrollFlags::NONE, |ui| {
            let id = ui.widget_id("duplicate");
            assert_eq!(id, ui.widget_id("duplicate"));
            ui.button("duplicate");
            ui.button("duplicate");
            ui.child("child", [160., 80.], ScrollFlags::NONE, |ui| {
                ui.button("duplicate");
            });
        });
        c.render();
        assert_eq!(c.duplicate_ids().len(), 1);
        let d = c.duplicate_ids()[0];
        assert_ne!(d.first, d.second);
        c.new_frame(0.016);
        c.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.button("duplicate");
        });
        c.render();
        assert!(c.duplicate_ids().is_empty());
        c.set_duplicate_id_diagnostics(false);
        c.new_frame(0.016);
        c.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.button("duplicate");
            ui.button("duplicate");
        });
        c.render();
        assert!(c.duplicate_ids().is_empty());
    }
    #[test]
    fn overflow_is_bounded_and_frame_local() {
        let mut d = DuplicateIds {
            enabled: true,
            ..DuplicateIds::default()
        };
        for _ in 0..8200 {
            d.register(Id(1), [0.; 4]);
        }
        assert_eq!(d.duplicates.len(), 8192);
        assert!(d.overflow);
        d.reset();
        assert!(!d.overflow);
        assert!(d.duplicates.is_empty());
    }
}
