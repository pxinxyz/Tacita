#[cfg(any(windows, test))]
use restricted_gui::editing::MAX_EDIT_BYTES;
use restricted_gui::{context::Context, editing::ClipboardAction};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    Unavailable,
    InvalidText,
    AllocationFailed,
}
#[cfg(any(windows, test))]
fn encode(text: &str) -> Result<Vec<u16>, ClipboardError> {
    if text.len() > MAX_EDIT_BYTES || text.contains('\0') {
        return Err(ClipboardError::InvalidText);
    }
    Ok(text.encode_utf16().chain(core::iter::once(0)).collect())
}
#[cfg(any(windows, test))]
fn decode(units: &[u16]) -> Result<String, ClipboardError> {
    let end = units
        .iter()
        .position(|u| *u == 0)
        .ok_or(ClipboardError::InvalidText)?;
    let result = String::from_utf16(&units[..end]).map_err(|_| ClipboardError::InvalidText)?;
    if result.len() > MAX_EDIT_BYTES {
        return Err(ClipboardError::InvalidText);
    }
    Ok(result)
}
pub fn process(context: &mut Context) -> Result<bool, ClipboardError> {
    let Some(request) = context.clipboard_request() else {
        return Ok(false);
    };
    let id = request.id;
    let result = match request.action {
        ClipboardAction::Paste => read_text(),
        ClipboardAction::Copy | ClipboardAction::Cut => {
            write_text(request.text).map(|_| String::new())
        }
    };
    let reply = result.as_ref().ok().map(String::as_str);
    context
        .complete_clipboard(id, reply)
        .map_err(|_| ClipboardError::InvalidText)?;
    result.map(|_| true)
}
#[cfg(not(windows))]
pub fn read_text() -> Result<String, ClipboardError> {
    Err(ClipboardError::Unavailable)
}
#[cfg(not(windows))]
pub fn write_text(_text: &str) -> Result<(), ClipboardError> {
    Err(ClipboardError::Unavailable)
}
#[cfg(windows)]
mod windows {
    use super::*;
    type Handle = *mut core::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(owner: Handle) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn GetClipboardData(format: u32) -> Handle;
        fn SetClipboardData(format: u32, memory: Handle) -> Handle;
        fn GetActiveWindow() -> Handle;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalAlloc(flags: u32, size: usize) -> Handle;
        fn GlobalFree(memory: Handle) -> Handle;
        fn GlobalLock(memory: Handle) -> Handle;
        fn GlobalUnlock(memory: Handle) -> i32;
        fn GlobalSize(memory: Handle) -> usize;
    }
    struct Open;
    impl Drop for Open {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
    struct Locked(Handle);
    impl Drop for Locked {
        fn drop(&mut self) {
            unsafe {
                GlobalUnlock(self.0);
            }
        }
    }
    struct Memory(Handle);
    impl Drop for Memory {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    GlobalFree(self.0);
                }
            }
        }
    }
    pub fn read() -> Result<String, ClipboardError> {
        unsafe {
            if OpenClipboard(core::ptr::null_mut()) == 0 {
                return Err(ClipboardError::Unavailable);
            }
            let _open = Open;
            let memory = GetClipboardData(13);
            if memory.is_null() {
                return Err(ClipboardError::Unavailable);
            }
            let bytes = GlobalSize(memory);
            if bytes < 2 || !bytes.is_multiple_of(2) || bytes > MAX_EDIT_BYTES * 2 + 2 {
                return Err(ClipboardError::InvalidText);
            }
            let data = GlobalLock(memory).cast::<u16>();
            if data.is_null() {
                return Err(ClipboardError::Unavailable);
            }
            let _locked = Locked(memory);
            decode(core::slice::from_raw_parts(data, bytes / 2))
        }
    }
    pub fn write(text: &str) -> Result<(), ClipboardError> {
        let encoded = encode(text)?;
        unsafe {
            let owner = GetActiveWindow();
            if owner.is_null() {
                return Err(ClipboardError::Unavailable);
            }
            let mut memory = Memory(GlobalAlloc(2, encoded.len() * 2));
            if memory.0.is_null() {
                return Err(ClipboardError::AllocationFailed);
            }
            let data = GlobalLock(memory.0).cast::<u16>();
            if data.is_null() {
                return Err(ClipboardError::AllocationFailed);
            }
            core::ptr::copy_nonoverlapping(encoded.as_ptr(), data, encoded.len());
            GlobalUnlock(memory.0);
            if OpenClipboard(owner) == 0 {
                return Err(ClipboardError::Unavailable);
            }
            let _open = Open;
            if EmptyClipboard() == 0 || SetClipboardData(13, memory.0).is_null() {
                return Err(ClipboardError::Unavailable);
            }
            memory.0 = core::ptr::null_mut();
            Ok(())
        }
    }
}
#[cfg(windows)]
pub fn read_text() -> Result<String, ClipboardError> {
    windows::read()
}
#[cfg(windows)]
pub fn write_text(text: &str) -> Result<(), ClipboardError> {
    windows::write(text)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_roundtrip_and_malformed_clipboard_rejection() {
        let text = "e\u{301} 👩‍💻 אבג 日本";
        assert_eq!(decode(&encode(text).unwrap()).unwrap(), text);
        assert_eq!(decode(&[0xd800, 0]), Err(ClipboardError::InvalidText));
        assert_eq!(decode(&[65]), Err(ClipboardError::InvalidText));
        assert_eq!(encode("a\0b"), Err(ClipboardError::InvalidText));
        assert_eq!(
            encode(&"a".repeat(MAX_EDIT_BYTES + 1)),
            Err(ClipboardError::InvalidText)
        );
    }
    #[test]
    fn no_request_never_touches_native_clipboard() {
        let mut c = Context::new(include_bytes!("../../core/tests/assets/DejaVuSans.ttf")).unwrap();
        assert_eq!(process(&mut c), Ok(false));
    }
}
