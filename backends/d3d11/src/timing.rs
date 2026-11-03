use crate::{BackendError, Context, Device};
use windows::{
    core::Interface,
    Win32::{Foundation::S_OK, Graphics::Direct3D11::*},
};
struct Slot {
    disjoint: ID3D11Query,
    start: ID3D11Query,
    end: ID3D11Query,
    pending: bool,
}
pub(crate) struct GpuTimer {
    slots: Vec<Slot>,
    active: Option<usize>,
    latest: Option<f64>,
}
fn query(device: &Device, kind: D3D11_QUERY) -> Result<ID3D11Query, BackendError> {
    let mut out = None;
    unsafe {
        device.CreateQuery(
            &D3D11_QUERY_DESC {
                Query: kind,
                MiscFlags: 0,
            },
            Some(&mut out),
        )
    }
    .map_err(|e| BackendError::StateCreation("timestamp query", e))?;
    crate::compositor::required(out)
}
unsafe fn ready<T>(ctx: &Context, query: &ID3D11Query, out: &mut T) -> bool {
    unsafe {
        (ctx.vtable().GetData)(
            ctx.as_raw(),
            query.as_raw(),
            (out as *mut T).cast(),
            core::mem::size_of::<T>() as u32,
            D3D11_ASYNC_GETDATA_DONOTFLUSH.0 as u32,
        ) == S_OK
    }
}
impl GpuTimer {
    pub fn new(device: &Device) -> Result<Self, BackendError> {
        let mut slots = Vec::new();
        for _ in 0..3 {
            slots.push(Slot {
                disjoint: query(device, D3D11_QUERY_TIMESTAMP_DISJOINT)?,
                start: query(device, D3D11_QUERY_TIMESTAMP)?,
                end: query(device, D3D11_QUERY_TIMESTAMP)?,
                pending: false,
            });
        }
        Ok(Self {
            slots,
            active: None,
            latest: None,
        })
    }
    pub fn poll(&mut self, ctx: &Context) {
        for slot in &mut self.slots {
            if !slot.pending {
                continue;
            }
            let mut disjoint = D3D11_QUERY_DATA_TIMESTAMP_DISJOINT::default();
            let mut start = 0u64;
            let mut end = 0u64;
            if unsafe {
                ready(ctx, &slot.disjoint, &mut disjoint)
                    && ready(ctx, &slot.start, &mut start)
                    && ready(ctx, &slot.end, &mut end)
            } {
                slot.pending = false;
                if !disjoint.Disjoint.as_bool() && disjoint.Frequency > 0 && end >= start {
                    self.latest = Some((end - start) as f64 * 1000. / disjoint.Frequency as f64);
                }
            }
        }
    }
    pub fn begin(&mut self, ctx: &Context) {
        self.poll(ctx);
        self.active = self.slots.iter().position(|s| !s.pending);
        if let Some(i) = self.active {
            let slot = &mut self.slots[i];
            unsafe {
                ctx.Begin(&slot.disjoint);
                ctx.End(&slot.start);
            }
            slot.pending = true;
        }
    }
    pub fn end(&mut self, ctx: &Context) {
        if let Some(i) = self.active.take() {
            let slot = &self.slots[i];
            unsafe {
                ctx.End(&slot.end);
                ctx.End(&slot.disjoint);
            }
        }
    }
    pub fn latest(&self) -> Option<f64> {
        self.latest
    }
}
