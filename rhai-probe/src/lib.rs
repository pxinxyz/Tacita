#![no_std]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc};
use core::cell::{RefCell, RefMut};
use tacita_sdk::{Gui, ScrollFlags};
use rhai::{CallFnOptions, Dynamic, Engine, EvalAltResult, Map, ParseError, Scope, AST};

#[derive(Clone)]
struct ScriptUi(Rc<RefCell<Gui>>);

pub struct Probe {
    ui: ScriptUi,
    engine: Engine,
    ast: AST,
    scope: Scope<'static>,
    state: Dynamic,
}

impl Probe {
    pub fn new(gui: Gui, source: &str) -> Result<Self, ParseError> {
        let mut engine = Engine::new();
        engine.set_max_operations(10_000);
        engine.set_max_call_levels(16);
        engine.register_type_with_name::<ScriptUi>("Ui");
        engine.register_fn("begin_window", |ui: &mut ScriptUi, title: &str| {
            ui.0.borrow_mut().begin_window(title, ScrollFlags::NONE);
        });
        engine.register_fn("end_window", |ui: &mut ScriptUi| {
            ui.0.borrow_mut().end_window();
        });
        engine.register_fn("label", |ui: &mut ScriptUi, text: &str| {
            ui.0.borrow_mut().label(text);
        });
        engine.register_fn(
            "checkbox",
            |ui: &mut ScriptUi, label: &str, mut value: bool| {
                ui.0.borrow_mut().checkbox(label, &mut value);
                value
            },
        );
        engine.register_fn(
            "slider",
            |ui: &mut ScriptUi, label: &str, mut value: f32, min: f32, max: f32| {
                ui.0.borrow_mut().slider_float(label, &mut value, min..=max);
                value
            },
        );
        engine.register_fn("button", |ui: &mut ScriptUi, label: &str| {
            ui.0.borrow_mut().button(label)
        });
        let ast = engine.compile(source)?;
        let mut state = Map::new();
        state.insert("enabled".into(), false.into());
        state.insert("gain".into(), 0.5_f32.into());
        state.insert("frames".into(), 0_i64.into());
        Ok(Self {
            ui: ScriptUi(Rc::new(RefCell::new(gui))),
            engine,
            ast,
            scope: Scope::new(),
            state: state.into(),
        })
    }

    pub fn gui(&self) -> RefMut<'_, Gui> {
        self.ui.0.borrow_mut()
    }

    pub fn state(&self) -> &Dynamic {
        &self.state
    }

    pub fn draw(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.engine.call_fn_with_options(
            CallFnOptions::new()
                .eval_ast(false)
                .bind_this_ptr(&mut self.state),
            &mut self.scope,
            &self.ast,
            "draw",
            (self.ui.clone(),),
        )
    }
}
