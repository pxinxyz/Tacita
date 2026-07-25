use alloc::vec::Vec;

// FNV-1a: the hash of choice for people who read one Usenet post from 2003 and never looked back. It works. Don't ask.
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = seed;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u64);

pub(crate) struct IdStack {
    seeds: Vec<u64>,
}

impl IdStack {
    pub(crate) fn new() -> Self {
        Self {
            seeds: alloc::vec![FNV_OFFSET_BASIS],
        }
    }

    pub(crate) fn push(&mut self, label: &str) {
        let seed = *self.seeds.last().expect("root seed is never removed");
        self.seeds
            .push(fnv1a(seed, split_label(label).1.as_bytes()));
    }

    pub(crate) fn pop(&mut self) {
        assert!(self.seeds.len() > 1, "IdStack::pop without a matching push");
        self.seeds.pop();
    }

    pub(crate) fn push_id(&mut self, id: Id) {
        self.seeds.push(id.0);
    }

    pub(crate) fn id(&self, label: &str) -> Id {
        let seed = *self.seeds.last().expect("root seed is never removed");
        Id(fnv1a(seed, split_label(label).1.as_bytes()))
    }
}

pub(crate) fn split_label(label: &str) -> (&str, &str) {
    match label.find("##") {
        Some(pos) => (&label[..pos], label),
        None => (label, label),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_label_cases() {
        assert_eq!(split_label("Name##a"), ("Name", "Name##a"));
        assert_eq!(split_label("##a"), ("", "##a"));
        assert_eq!(split_label("###a"), ("", "###a"));
        assert_eq!(split_label("Name"), ("Name", "Name"));
    }

    #[test]
    fn same_label_different_scopes_give_different_ids() {
        let mut a = IdStack::new();
        a.push("Window A");
        let mut b = IdStack::new();
        b.push("Window B");

        assert_ne!(a.id("Button"), b.id("Button"));
    }

    #[test]
    fn identical_stacks_give_identical_ids() {
        let mut a = IdStack::new();
        a.push("Window");
        let mut b = IdStack::new();
        b.push("Window");

        assert_eq!(a.id("Button"), b.id("Button"));
        assert_ne!(IdStack::new().id("Button"), a.id("Button"));
    }

    #[test]
    fn push_pop_restores_previous_id() {
        let mut stack = IdStack::new();
        let before = stack.id("x");
        stack.push("Window");
        stack.pop();
        assert_eq!(before, stack.id("x"));
    }

    #[test]
    fn hidden_id_suffix_changes_id_but_not_display() {
        let stack = IdStack::new();
        assert_ne!(stack.id("Name##a"), stack.id("Name##b"));
        assert_ne!(stack.id("Name##a"), stack.id("Name"));
        assert_eq!(split_label("Name##a").0, split_label("Name##b").0);
    }

    #[test]
    #[should_panic]
    fn pop_without_matching_push_panics() {
        IdStack::new().pop();
    }
}
