use super::*;
use restricted_gui::accessibility::{
    AccessibilityActionKind as Kind, AccessibilityActions as Actions, AccessibilityRole as Role,
    AccessibilityState as Flags,
};
use std::hash::{Hash, Hasher};
use std::{
    ffi::c_void,
    marker::PhantomData,
    rc::Rc,
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
type HResult = i32;
const OK: i32 = 0;
const FALSE: i32 = 1;
const INVALID: i32 = 0x80070057u32 as i32;
const POINTER: i32 = 0x80004003u32 as i32;
const NO_INTERFACE: i32 = 0x80004002u32 as i32;
const UNAVAILABLE: i32 = 0x800401FDu32 as i32;
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}
const UNKNOWN: Guid = Guid {
    a: 0,
    b: 0,
    c: 0,
    d: [0xc0, 0, 0, 0, 0, 0, 0, 0x46],
};
const DISPATCH: Guid = Guid {
    a: 0x00020400,
    b: 0,
    c: 0,
    d: [0xc0, 0, 0, 0, 0, 0, 0, 0x46],
};
const ACCESSIBLE: Guid = Guid {
    a: 0x618736e0,
    b: 0x3c3d,
    c: 0x11cf,
    d: [0x81, 0x0c, 0, 0xaa, 0, 0x38, 0x9b, 0x71],
};
#[repr(C)]
#[derive(Clone, Copy)]
union Data {
    integer: i32,
    number: f64,
    pointer: *mut c_void,
    padding: [usize; 2],
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Variant {
    kind: u16,
    reserved: [u16; 3],
    data: Data,
}
impl Variant {
    fn empty() -> Self {
        Self {
            kind: 0,
            reserved: [0; 3],
            data: Data { padding: [0; 2] },
        }
    }
    fn int(v: i32) -> Self {
        Self {
            kind: 3,
            reserved: [0; 3],
            data: Data { integer: v },
        }
    }
    fn object(p: *mut Object) -> Self {
        Self {
            kind: 9,
            reserved: [0; 3],
            data: Data { pointer: p.cast() },
        }
    }
}
#[repr(C)]
struct DispatchParams {
    args: *mut Variant,
    named: *mut i32,
    count: u32,
    named_count: u32,
}
struct Shared {
    snapshot: Mutex<Snapshot>,
    actions: Mutex<Vec<AccessibilityAction>>,
    window: AtomicUsize,
}
#[repr(C)]
struct Object {
    table: &'static VTable,
    refs: AtomicU32,
    shared: Arc<Shared>,
    id: Option<Id>,
}
impl Object {
    fn new(shared: Arc<Shared>, id: Option<Id>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            table: &TABLE,
            refs: AtomicU32::new(1),
            shared,
            id,
        }))
    }
}
#[repr(C)]
struct VTable {
    query: unsafe extern "system" fn(*mut Object, *const Guid, *mut *mut c_void) -> HResult,
    add: unsafe extern "system" fn(*mut Object) -> u32,
    release: unsafe extern "system" fn(*mut Object) -> u32,
    type_count: unsafe extern "system" fn(*mut Object, *mut u32) -> HResult,
    type_info: unsafe extern "system" fn(*mut Object, u32, u32, *mut *mut c_void) -> HResult,
    names: unsafe extern "system" fn(
        *mut Object,
        *const Guid,
        *const *const u16,
        u32,
        u32,
        *mut i32,
    ) -> HResult,
    invoke: unsafe extern "system" fn(
        *mut Object,
        i32,
        *const Guid,
        u32,
        u16,
        *mut DispatchParams,
        *mut Variant,
        *mut c_void,
        *mut u32,
    ) -> HResult,
    parent: unsafe extern "system" fn(*mut Object, *mut *mut Object) -> HResult,
    count: unsafe extern "system" fn(*mut Object, *mut i32) -> HResult,
    child: unsafe extern "system" fn(*mut Object, Variant, *mut *mut Object) -> HResult,
    name: unsafe extern "system" fn(*mut Object, Variant, *mut *mut u16) -> HResult,
    value: unsafe extern "system" fn(*mut Object, Variant, *mut *mut u16) -> HResult,
    description: unsafe extern "system" fn(*mut Object, Variant, *mut *mut u16) -> HResult,
    role: unsafe extern "system" fn(*mut Object, Variant, *mut Variant) -> HResult,
    state: unsafe extern "system" fn(*mut Object, Variant, *mut Variant) -> HResult,
    help: unsafe extern "system" fn(*mut Object, Variant, *mut *mut u16) -> HResult,
    help_topic: unsafe extern "system" fn(*mut Object, *mut *mut u16, Variant, *mut i32) -> HResult,
    shortcut: unsafe extern "system" fn(*mut Object, Variant, *mut *mut u16) -> HResult,
    focus: unsafe extern "system" fn(*mut Object, *mut Variant) -> HResult,
    selection: unsafe extern "system" fn(*mut Object, *mut Variant) -> HResult,
    default_action: unsafe extern "system" fn(*mut Object, Variant, *mut *mut u16) -> HResult,
    select: unsafe extern "system" fn(*mut Object, i32, Variant) -> HResult,
    location: unsafe extern "system" fn(
        *mut Object,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        Variant,
    ) -> HResult,
    navigate: unsafe extern "system" fn(*mut Object, i32, Variant, *mut Variant) -> HResult,
    hit: unsafe extern "system" fn(*mut Object, i32, i32, *mut Variant) -> HResult,
    perform: unsafe extern "system" fn(*mut Object, Variant) -> HResult,
    put_name: unsafe extern "system" fn(*mut Object, Variant, *const u16) -> HResult,
    put_value: unsafe extern "system" fn(*mut Object, Variant, *const u16) -> HResult,
}
static TABLE: VTable = VTable {
    query,
    add,
    release,
    type_count,
    type_info,
    names,
    invoke,
    parent,
    count,
    child,
    name,
    value,
    description: empty_text,
    role,
    state,
    help: empty_text,
    help_topic,
    shortcut: empty_text,
    focus,
    selection,
    default_action,
    select,
    location,
    navigate,
    hit,
    perform,
    put_name,
    put_value,
};
#[link(name = "oleacc")]
extern "system" {
    fn LresultFromObject(iid: *const Guid, wparam: usize, object: *mut c_void) -> isize;
}
#[link(name = "oleaut32")]
extern "system" {
    fn SysAllocStringLen(s: *const u16, len: u32) -> *mut u16;
    fn SysStringLen(s: *const u16) -> u32;
}
#[link(name = "ole32")]
extern "system" {
    fn OleInitialize(p: *mut c_void) -> i32;
    fn OleUninitialize();
}
type Subclass = unsafe extern "system" fn(isize, u32, usize, isize, usize, usize) -> isize;
#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(hwnd: isize, proc: Subclass, id: usize, data: usize) -> i32;
    fn RemoveWindowSubclass(hwnd: isize, proc: Subclass, id: usize) -> i32;
    fn DefSubclassProc(hwnd: isize, msg: u32, w: usize, l: isize) -> isize;
}
#[link(name = "user32")]
extern "system" {
    fn NotifyWinEvent(event: u32, hwnd: isize, object: i32, child: i32);
    fn InvalidateRect(hwnd: isize, rect: *const c_void, erase: i32) -> i32;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
}

unsafe extern "system" fn query(p: *mut Object, iid: *const Guid, out: *mut *mut c_void) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = std::ptr::null_mut();
    if iid.is_null() {
        return POINTER;
    }
    if [UNKNOWN, DISPATCH, ACCESSIBLE].contains(&*iid) {
        *out = p.cast();
        add(p);
        OK
    } else {
        NO_INTERFACE
    }
}
unsafe extern "system" fn add(p: *mut Object) -> u32 {
    (&*p).refs.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn release(p: *mut Object) -> u32 {
    let n = (&*p).refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if n == 0 {
        drop(Box::from_raw(p));
    }
    n
}
unsafe extern "system" fn type_count(_: *mut Object, out: *mut u32) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = 0;
    OK
}
unsafe extern "system" fn type_info(_: *mut Object, _: u32, _: u32, out: *mut *mut c_void) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = std::ptr::null_mut();
    0x8002000bu32 as i32
}
fn dispid(s: &str) -> Option<i32> {
    Some(match s.to_ascii_lowercase().as_str() {
        "accparent" => -5000,
        "accchildcount" => -5001,
        "accchild" => -5002,
        "accname" => -5003,
        "accvalue" => -5004,
        "accdescription" => -5005,
        "accrole" => -5006,
        "accstate" => -5007,
        "acchelp" => -5008,
        "acchelptopic" => -5009,
        "acckeyboardshortcut" => -5010,
        "accfocus" => -5011,
        "accselection" => -5012,
        "accdefaultaction" => -5013,
        "accselect" => -5014,
        "acclocation" => -5015,
        "accnavigate" => -5016,
        "acchittest" => -5017,
        "accdodefaultaction" => -5018,
        _ => return None,
    })
}
unsafe extern "system" fn names(
    _: *mut Object,
    _: *const Guid,
    input: *const *const u16,
    n: u32,
    _: u32,
    out: *mut i32,
) -> i32 {
    if input.is_null() || out.is_null() {
        return POINTER;
    }
    if n == 0 || n > 32 {
        return INVALID;
    }
    for i in 0..n as usize {
        let p = *input.add(i);
        if p.is_null() {
            return POINTER;
        }
        let mut len = 0;
        while len < 64 && *p.add(len) != 0 {
            len += 1;
        }
        if len == 64 {
            return INVALID;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, len));
        let Some(id) = dispid(&s) else {
            return 0x80020006u32 as i32;
        };
        *out.add(i) = id;
    }
    OK
}
unsafe extern "system" fn invoke(
    p: *mut Object,
    id: i32,
    _: *const Guid,
    _: u32,
    flags: u16,
    args: *mut DispatchParams,
    out: *mut Variant,
    _: *mut c_void,
    _: *mut u32,
) -> i32 {
    if args.is_null() {
        return POINTER;
    }
    if (*args).count > 2 || ((*args).count > 0 && (*args).args.is_null()) {
        return INVALID;
    }
    let v = if (*args).count > 0 {
        *(*args).args.add((*args).count as usize - 1)
    } else {
        Variant::int(0)
    };
    if flags & 4 != 0 && id == -5004 && (*args).count == 2 {
        let text = *(*args).args;
        if text.kind != 8 {
            return INVALID;
        }
        return put_value(p, v, text.data.pointer.cast());
    }
    if id == -5018 {
        return perform(p, v);
    }
    if id == -5014 && (*args).count == 2 {
        let f = *(*args).args.add(1);
        if f.kind != 3 {
            return INVALID;
        }
        return select(p, f.data.integer, *(*args).args);
    }
    if out.is_null() {
        return POINTER;
    }
    *out = Variant::empty();
    match id {
        -5000 | -5002 => {
            let mut child_obj = std::ptr::null_mut();
            let hr = if id == -5000 {
                parent(p, &mut child_obj)
            } else {
                child(p, v, &mut child_obj)
            };
            if hr == OK {
                *out = Variant::object(child_obj);
            }
            hr
        }
        -5001 => {
            let mut n = 0;
            let hr = count(p, &mut n);
            *out = Variant::int(n);
            hr
        }
        -5003 | -5004 | -5005 | -5008 | -5010 | -5013 => {
            let mut bstr = std::ptr::null_mut();
            let hr = match id {
                -5003 => name(p, v, &mut bstr),
                -5004 => value(p, v, &mut bstr),
                -5013 => default_action(p, v, &mut bstr),
                _ => empty_text(p, v, &mut bstr),
            };
            if hr == OK {
                *out = Variant {
                    kind: 8,
                    reserved: [0; 3],
                    data: Data {
                        pointer: bstr.cast(),
                    },
                };
            }
            hr
        }
        -5006 => role(p, v, out),
        -5007 => state(p, v, out),
        -5011 => focus(p, out),
        -5012 => selection(p, out),
        -5016 if (*args).count == 2 => {
            let dir = *(*args).args.add(1);
            if dir.kind != 3 {
                return INVALID;
            }
            navigate(p, dir.data.integer, *(*args).args, out)
        }
        -5017 if (*args).count == 2 => {
            let a = *(*args).args;
            let b = *(*args).args.add(1);
            if a.kind != 3 || b.kind != 3 {
                return INVALID;
            }
            hit(p, b.data.integer, a.data.integer, out)
        }
        _ => 0x80020003u32 as i32,
    }
}
unsafe fn resolve(p: *mut Object, v: Variant, s: &Snapshot) -> Result<Option<Id>, i32> {
    if (&*p).shared.window.load(Ordering::Acquire) == 0 {
        return Err(UNAVAILABLE);
    }
    if v.kind != 3 {
        return Err(INVALID);
    }
    let n = v.data.integer;
    if n == 0 {
        if let Some(id) = (*p).id {
            if s.node(id).is_none() {
                return Err(UNAVAILABLE);
            }
        }
        return Ok((*p).id);
    }
    if n < 0 {
        return Err(INVALID);
    }
    s.children((*p).id)
        .nth(n as usize - 1)
        .map(|node| Some(node.semantic.id))
        .ok_or(INVALID)
}
unsafe fn owned(p: *mut Object, id: Option<Id>) -> *mut Object {
    Object::new((&*p).shared.clone(), id)
}
unsafe extern "system" fn parent(p: *mut Object, out: *mut *mut Object) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = std::ptr::null_mut();
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if resolve(p, Variant::int(0), &s).is_err() {
        return UNAVAILABLE;
    }
    if let Some(id) = (*p).id {
        *out = owned(p, s.parent(id));
        OK
    } else {
        FALSE
    }
}
unsafe extern "system" fn count(p: *mut Object, out: *mut i32) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = 0;
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if resolve(p, Variant::int(0), &s).is_err() {
        return UNAVAILABLE;
    }
    *out = s.children((*p).id).count() as i32;
    OK
}
unsafe extern "system" fn child(p: *mut Object, v: Variant, out: *mut *mut Object) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = std::ptr::null_mut();
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    match resolve(p, v, &s) {
        Ok(id) => {
            *out = owned(p, id);
            OK
        }
        Err(hr) => hr,
    }
}
unsafe fn bstr(text: &str, out: *mut *mut u16) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = std::ptr::null_mut();
    if text.is_empty() {
        return FALSE;
    }
    let utf16: Vec<u16> = text.encode_utf16().collect();
    *out = SysAllocStringLen(utf16.as_ptr(), utf16.len() as u32);
    if (*out).is_null() {
        0x8007000eu32 as i32
    } else {
        OK
    }
}
unsafe fn text_property(p: *mut Object, v: Variant, out: *mut *mut u16, which: u32) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = std::ptr::null_mut();
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let id = match resolve(p, v, &s) {
        Ok(id) => id,
        Err(hr) => return hr,
    };
    if let Some(n) = id.and_then(|id| s.node(id)) {
        match which {
            0 => bstr(&n.name, out),
            1 => bstr(&n.value, out),
            2 if n.semantic.actions & Actions::ACTIVATE != 0 => bstr("activate", out),
            _ => FALSE,
        }
    } else if which == 0 {
        bstr("restricted gui preview", out)
    } else {
        FALSE
    }
}
unsafe extern "system" fn name(p: *mut Object, v: Variant, out: *mut *mut u16) -> i32 {
    text_property(p, v, out, 0)
}
unsafe extern "system" fn value(p: *mut Object, v: Variant, out: *mut *mut u16) -> i32 {
    text_property(p, v, out, 1)
}
unsafe extern "system" fn default_action(p: *mut Object, v: Variant, out: *mut *mut u16) -> i32 {
    text_property(p, v, out, 2)
}
unsafe extern "system" fn empty_text(p: *mut Object, v: Variant, out: *mut *mut u16) -> i32 {
    text_property(p, v, out, 3)
}
fn role_code(r: Role) -> i32 {
    match r {
        Role::Group => 0x14,
        Role::Window => 9,
        Role::Text => 0x29,
        Role::Button => 0x2b,
        Role::CheckBox => 0x2c,
        Role::RadioButton => 0x2d,
        Role::Slider => 0x33,
        Role::TextInput => 0x2a,
        Role::ComboBox => 0x2e,
        Role::Tab => 0x25,
        Role::TreeItem => 0x24,
        Role::ProgressBar => 0x30,
        Role::Image => 0x28,
        Role::List => 0x21,
        Role::ListItem => 0x22,
        Role::MenuItem => 0x0c,
    }
}
fn state_code(n: &Node) -> i32 {
    let s = n.semantic.states;
    let mut r = 0;
    for (flag, msaa) in [
        (Flags::DISABLED, 1),
        (Flags::FOCUSED, 4),
        (Flags::CHECKED, 16),
        (Flags::SELECTED, 2),
        (Flags::EXPANDED, 0x200),
        (Flags::READ_ONLY, 0x40),
        (Flags::HIDDEN, 0x8000),
        (Flags::BLOCKED, 1),
    ] {
        if s & flag != 0 {
            r |= msaa;
        }
    }
    if n.semantic.actions & Actions::FOCUS != 0 {
        r |= 0x100000;
    }
    if n.semantic.role == Role::TreeItem && s & Flags::EXPANDED == 0 {
        r |= 0x400;
    }
    r
}
unsafe fn integer_property(p: *mut Object, v: Variant, out: *mut Variant, states: bool) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = Variant::empty();
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let id = match resolve(p, v, &s) {
        Ok(id) => id,
        Err(hr) => return hr,
    };
    let n = id.and_then(|id| s.node(id));
    let code = if states {
        n.map_or(0, state_code)
    } else {
        n.map_or(0x0a, |n| role_code(n.semantic.role))
    };
    *out = Variant::int(code);
    OK
}
unsafe extern "system" fn role(p: *mut Object, v: Variant, out: *mut Variant) -> i32 {
    integer_property(p, v, out, false)
}
unsafe extern "system" fn state(p: *mut Object, v: Variant, out: *mut Variant) -> i32 {
    integer_property(p, v, out, true)
}
unsafe extern "system" fn help_topic(
    p: *mut Object,
    out: *mut *mut u16,
    v: Variant,
    topic: *mut i32,
) -> i32 {
    if topic.is_null() {
        return POINTER;
    }
    *topic = -1;
    empty_text(p, v, out)
}
unsafe fn find_flag(p: *mut Object, out: *mut Variant, flag: u32) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = Variant::empty();
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if resolve(p, Variant::int(0), &s).is_err() {
        return UNAVAILABLE;
    }
    if let Some(n) = s
        .nodes
        .iter()
        .find(|n| s.contains((*p).id, n.semantic.id) && n.semantic.states & flag != 0)
    {
        *out = Variant::object(owned(p, Some(n.semantic.id)));
        OK
    } else {
        FALSE
    }
}
unsafe extern "system" fn focus(p: *mut Object, out: *mut Variant) -> i32 {
    find_flag(p, out, Flags::FOCUSED)
}
unsafe extern "system" fn selection(p: *mut Object, out: *mut Variant) -> i32 {
    find_flag(p, out, Flags::SELECTED)
}
unsafe fn queue(p: *mut Object, v: Variant, kind: Kind, number: f64) -> i32 {
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let id = match resolve(p, v, &s) {
        Ok(Some(id)) => id,
        Ok(None) => return FALSE,
        Err(hr) => return hr,
    };
    let n = s.node(id).unwrap();
    if !number.is_finite()
        || n.semantic.actions & kind as u32 == 0
        || n.semantic.states & (Flags::DISABLED | Flags::HIDDEN | Flags::BLOCKED) != 0
    {
        return INVALID;
    }
    let mut actions = (&*p)
        .shared
        .actions
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if actions.len() >= 64 {
        return FALSE;
    }
    actions.push(AccessibilityAction { id, kind, number });
    InvalidateRect(
        (&*p).shared.window.load(Ordering::Acquire) as isize,
        std::ptr::null(),
        0,
    );
    OK
}
unsafe extern "system" fn select(p: *mut Object, flags: i32, v: Variant) -> i32 {
    if flags != 1 {
        return INVALID;
    }
    queue(p, v, Kind::Focus, 0.)
}
unsafe extern "system" fn perform(p: *mut Object, v: Variant) -> i32 {
    queue(p, v, Kind::Activate, 0.)
}
unsafe extern "system" fn put_name(_: *mut Object, _: Variant, _: *const u16) -> i32 {
    INVALID
}
unsafe extern "system" fn put_value(p: *mut Object, v: Variant, text: *const u16) -> i32 {
    if text.is_null() {
        return POINTER;
    }
    let len = SysStringLen(text) as usize;
    if len > 64 {
        return INVALID;
    }
    let text = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
    let Ok(number) = text.parse::<f64>() else {
        return INVALID;
    };
    queue(p, v, Kind::SetNumber, number)
}
unsafe extern "system" fn location(
    p: *mut Object,
    x: *mut i32,
    y: *mut i32,
    w: *mut i32,
    h: *mut i32,
    v: Variant,
) -> i32 {
    if [x, y, w, h].iter().any(|p| p.is_null()) {
        return POINTER;
    }
    *x = 0;
    *y = 0;
    *w = 0;
    *h = 0;
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let id = match resolve(p, v, &s) {
        Ok(id) => id,
        Err(hr) => return hr,
    };
    let r = if let Some(n) = id.and_then(|id| s.node(id)) {
        n.semantic.rect
    } else {
        let mut roots = s.children(None);
        let Some(first) = roots.next() else {
            return FALSE;
        };
        roots.fold(first.semantic.rect, |r, n| {
            let b = n.semantic.rect;
            [
                r[0].min(b[0]),
                r[1].min(b[1]),
                r[2].max(b[2]),
                r[3].max(b[3]),
            ]
        })
    };
    *x = r[0].floor() as i32;
    *y = r[1].floor() as i32;
    *w = (r[2].ceil() as i64 - i64::from(*x)).clamp(0, i64::from(i32::MAX)) as i32;
    *h = (r[3].ceil() as i64 - i64::from(*y)).clamp(0, i64::from(i32::MAX)) as i32;
    OK
}
unsafe extern "system" fn navigate(p: *mut Object, dir: i32, v: Variant, out: *mut Variant) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = Variant::empty();
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let id = match resolve(p, v, &s) {
        Ok(id) => id,
        Err(hr) => return hr,
    };
    let target = match dir {
        7 => s.children(id).next().map(|n| n.semantic.id),
        8 => s.children(id).last().map(|n| n.semantic.id),
        5 | 6 => {
            let parent = id.and_then(|id| s.parent(id));
            let siblings: Vec<_> = s.children(parent).map(|n| n.semantic.id).collect();
            id.and_then(|id| siblings.iter().position(|n| *n == id))
                .and_then(|i| {
                    if dir == 5 {
                        siblings.get(i + 1).copied()
                    } else {
                        i.checked_sub(1).and_then(|i| siblings.get(i).copied())
                    }
                })
        }
        _ => return INVALID,
    };
    if let Some(id) = target {
        *out = Variant::object(owned(p, Some(id)));
        OK
    } else {
        FALSE
    }
}
unsafe extern "system" fn hit(p: *mut Object, x: i32, y: i32, out: *mut Variant) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    *out = Variant::empty();
    let s = (&*p)
        .shared
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if resolve(p, Variant::int(0), &s).is_err() {
        return UNAVAILABLE;
    }
    if let Some(n) = s.nodes.iter().rev().find(|n| {
        let r = n.semantic.rect;
        s.contains((*p).id, n.semantic.id)
            && n.semantic.states & (Flags::HIDDEN | Flags::BLOCKED) == 0
            && (x as f32) >= r[0]
            && (y as f32) >= r[1]
            && (x as f32) < r[2]
            && (y as f32) < r[3]
    }) {
        *out = Variant::object(owned(p, Some(n.semantic.id)));
        OK
    } else {
        FALSE
    }
}

pub struct ScreenReaderBridge {
    shared: Arc<Shared>,
    root: *mut Object,
    ole: bool,
    _thread: PhantomData<Rc<()>>,
}
unsafe extern "system" fn subclass(
    hwnd: isize,
    msg: u32,
    w: usize,
    l: isize,
    id: usize,
    data: usize,
) -> isize {
    let root = data as *mut Object;
    if msg == 0x3d && (l as u32) == 0xfffffffcu32 {
        return LresultFromObject(&ACCESSIBLE, w, root.cast());
    }
    if msg == 0x82 {
        (&*root).shared.window.store(0, Ordering::Release);
        RemoveWindowSubclass(hwnd, subclass, id);
    }
    DefSubclassProc(hwnd, msg, w, l)
}
impl ScreenReaderBridge {
    pub unsafe fn attach(hwnd: isize) -> Result<Self, &'static str> {
        if hwnd == 0 || GetWindowThreadProcessId(hwnd, std::ptr::null_mut()) != GetCurrentThreadId()
        {
            return Err("accessibility requires a live window on this thread");
        }
        let hr = OleInitialize(std::ptr::null_mut());
        if hr < 0 {
            return Err("OLE initialization failed");
        }
        let shared = Arc::new(Shared {
            snapshot: Mutex::new(Snapshot::default()),
            actions: Mutex::new(Vec::with_capacity(64)),
            window: AtomicUsize::new(hwnd as usize),
        });
        let root = Object::new(shared.clone(), None);
        if SetWindowSubclass(hwnd, subclass, root as usize, root as usize) == 0 {
            release(root);
            OleUninitialize();
            return Err("accessibility window attachment failed");
        }
        Ok(Self {
            shared,
            root,
            ole: true,
            _thread: PhantomData,
        })
    }
    pub fn publish(&mut self, ctx: &Context, origin: [f32; 2], scale: f32) {
        let hwnd = self.shared.window.load(Ordering::Acquire) as isize;
        if hwnd == 0 {
            return;
        }
        let mut s = self
            .shared
            .snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let old_focus = s
            .nodes
            .iter()
            .find(|n| n.semantic.states & Flags::FOCUSED != 0)
            .map(|n| n.semantic.id);
        let before = fingerprints(&s);
        s.update(ctx, origin, scale);
        let focus = s
            .nodes
            .iter()
            .find(|n| n.semantic.states & Flags::FOCUSED != 0)
            .map(|n| n.semantic.id);
        let after = fingerprints(&s);
        drop(s);
        unsafe {
            if before.0 != after.0 {
                NotifyWinEvent(0x8004, hwnd, -4, 0);
            }
            if old_focus != focus {
                NotifyWinEvent(0x8005, hwnd, -4, 0);
            }
            if before.1 != after.1 {
                NotifyWinEvent(0x800a, hwnd, -4, 0);
            }
            if before.2 != after.2 {
                NotifyWinEvent(0x800e, hwnd, -4, 0);
            }
        }
    }
    pub fn drain_actions(&mut self, ctx: &mut Context) -> usize {
        let mut actions = self
            .shared
            .actions
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        apply_actions(ctx, actions.drain(..))
    }
}

fn fingerprints(s: &Snapshot) -> (u64, u64, u64) {
    let mut tree = std::collections::hash_map::DefaultHasher::new();
    let mut states = std::collections::hash_map::DefaultHasher::new();
    let mut values = std::collections::hash_map::DefaultHasher::new();
    for n in &s.nodes {
        n.semantic.id.0.hash(&mut tree);
        n.semantic.parent.hash(&mut tree);
        (n.semantic.role as u32).hash(&mut tree);
        n.name.hash(&mut tree);
        for bound in n.semantic.rect {
            bound.to_bits().hash(&mut tree);
        }
        n.semantic.states.hash(&mut states);
        n.semantic.actions.hash(&mut states);
        n.value.hash(&mut values);
    }
    (tree.finish(), states.finish(), values.finish())
}
impl Drop for ScreenReaderBridge {
    fn drop(&mut self) {
        unsafe {
            let hwnd = self.shared.window.swap(0, Ordering::AcqRel) as isize;
            if hwnd != 0 {
                RemoveWindowSubclass(hwnd, subclass, self.root as usize);
            }
            release(self.root);
            if self.ole {
                OleUninitialize();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shared() -> Arc<Shared> {
        let mut ctx =
            Context::new(include_bytes!("../../../core/tests/assets/DejaVuSans.ttf")).unwrap();
        ctx.set_accessibility_enabled(true);
        ctx.new_frame(0.016);
        ctx.ui()
            .window("first", restricted_gui::ScrollFlags::NONE, |ui| {
                ui.button("one");
            });
        ctx.ui()
            .window("second", restricted_gui::ScrollFlags::NONE, |ui| {
                ui.button("two");
            });
        ctx.render();
        let mut snapshot = Snapshot::default();
        snapshot.update(&ctx, [0.; 2], 1.);
        Arc::new(Shared {
            snapshot: Mutex::new(snapshot),
            actions: Mutex::new(Vec::new()),
            window: AtomicUsize::new(1),
        })
    }
    #[test]
    fn native_variant_and_vtable_match_x64_sdk_layout() {
        assert_eq!(std::mem::size_of::<Guid>(), 16);
        assert_eq!(std::mem::offset_of!(Variant, data), 8);
        assert_eq!(
            std::mem::size_of::<Variant>(),
            if cfg!(target_pointer_width = "64") {
                24
            } else {
                16
            }
        );
        assert_eq!(
            std::mem::size_of::<VTable>(),
            28 * std::mem::size_of::<usize>()
        );
        assert_eq!(std::mem::offset_of!(Object, table), 0);
    }
    #[test]
    fn child_enumeration_parent_and_detached_lifetime() {
        unsafe {
            let s = shared();
            let root = Object::new(s.clone(), None);
            let mut n = 0;
            assert_eq!(count(root, &mut n), OK);
            assert_eq!(n, 2);
            let mut window = std::ptr::null_mut();
            assert_eq!(child(root, Variant::int(1), &mut window), OK);
            assert_eq!(count(window, &mut n), OK);
            assert_eq!(n, 1);
            let mut leaf = std::ptr::null_mut();
            assert_eq!(child(window, Variant::int(1), &mut leaf), OK);
            let mut parent_obj = std::ptr::null_mut();
            assert_eq!(parent(leaf, &mut parent_obj), OK);
            assert_eq!((*parent_obj).id, (*window).id);
            release(parent_obj);
            let mut queried = std::ptr::null_mut();
            assert_eq!(query(leaf, &ACCESSIBLE, &mut queried), OK);
            assert_eq!(release(queried.cast()), 1);
            assert_eq!(child(window, Variant::int(-1), &mut parent_obj), INVALID);
            s.window.store(0, Ordering::Release);
            assert_eq!(count(leaf, &mut n), UNAVAILABLE);
            assert_eq!(n, 0);
            assert_eq!(release(root), 0);
            assert_eq!(release(window), 0);
            assert_eq!(release(leaf), 0);
        }
    }
    #[test]
    fn native_focus_and_hits_are_limited_to_object_subtree() {
        unsafe {
            let s = shared();
            let mut snapshot = s.snapshot.lock().unwrap();
            let second = snapshot.children(None).nth(1).unwrap().semantic.id;
            let focused = snapshot.children(Some(second)).next().unwrap().semantic.id;
            snapshot
                .nodes
                .iter_mut()
                .find(|n| n.semantic.id == focused)
                .unwrap()
                .semantic
                .states |= Flags::FOCUSED;
            let first = snapshot.children(None).next().unwrap().semantic.id;
            drop(snapshot);
            let p = Object::new(s, Some(first));
            let mut out = Variant::empty();
            assert_eq!(focus(p, &mut out), FALSE);
            assert_eq!(out.kind, 0);
            release(p);
        }
    }
}
