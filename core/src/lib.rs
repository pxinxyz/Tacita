#![no_std]

extern crate alloc;

pub mod accessibility;
pub mod host_requests;

#[cfg(test)]
extern crate std;

pub mod animation;
pub(crate) mod color;
pub mod context;
pub mod data;
pub mod diagnostics;
pub mod display;
pub mod drag_drop;
pub mod draw;
pub mod editing;
pub mod effects;
pub mod font;
pub mod icon;
pub mod id;
pub mod images;
pub mod input;
pub mod interaction;
pub mod layout;
pub mod panels;
pub mod paths;
pub mod persistence;
pub(crate) mod raster;
pub mod rendering;
pub(crate) mod scroll;
pub mod shapes;
pub mod style;
pub mod svg;
pub mod text;
pub mod theme;
pub mod ui;
pub(crate) mod widgets;

pub use scroll::{ScrollFlags, ScrollbarSkin, ScrollbarTrackClick};
