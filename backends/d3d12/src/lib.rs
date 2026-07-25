mod offscreen;
mod pipeline;
mod shader;
mod texture;

use std::fmt;

use restricted_gui::draw::{DrawData, TextureId};
use windows::Win32::Foundation::{CloseHandle, E_UNEXPECTED, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::{
    D3D12CreateDevice, ID3D12Fence, ID3D12GraphicsCommandList, ID3D12Resource,
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
    D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_FENCE_FLAG_NONE, D3D12_RESOURCE_STATES,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};

pub use offscreen::OffscreenTarget;
pub use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandQueue as Queue, ID3D12Device as Device,
};

#[derive(Debug)]
pub enum BackendError {
    DeviceCreation(windows::core::Error),
    CommandQueueCreation(windows::core::Error),
    FenceCreation(windows::core::Error),
    EventCreation(windows::core::Error),
    CommandAllocatorCreation(windows::core::Error),
    CommandListCreation(windows::core::Error),
    CommandRecording(&'static str, windows::core::Error),
    ShaderCompile(String),
    StateCreation(&'static str, windows::core::Error),
    TextureCreation(windows::core::Error),
    BufferCreation(windows::core::Error),
    DescriptorHeapCreation(windows::core::Error),
    MapFailed(windows::core::Error),
    SignalFailed(windows::core::Error),
    WaitFailed(String),
    ReservedTextureId(u64),
    InvalidTextureData(String),
    DescriptorHeapExhausted(u32),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::DeviceCreation(e) => write!(f, "D3D12 device creation failed: {e}"),
            BackendError::CommandQueueCreation(e) => {
                write!(f, "command queue creation failed: {e}")
            }
            BackendError::FenceCreation(e) => write!(f, "fence creation failed: {e}"),
            BackendError::EventCreation(e) => write!(f, "fence event creation failed: {e}"),
            BackendError::CommandAllocatorCreation(e) => {
                write!(f, "command allocator creation failed: {e}")
            }
            BackendError::CommandListCreation(e) => {
                write!(f, "command list creation failed: {e}")
            }
            BackendError::CommandRecording(what, e) => write!(f, "{what} failed: {e}"),
            BackendError::ShaderCompile(msg) => write!(f, "shader compilation failed: {msg}"),
            BackendError::StateCreation(what, e) => write!(f, "{what} creation failed: {e}"),
            BackendError::TextureCreation(e) => write!(f, "texture creation failed: {e}"),
            BackendError::BufferCreation(e) => write!(f, "buffer creation failed: {e}"),
            BackendError::DescriptorHeapCreation(e) => {
                write!(f, "descriptor heap creation failed: {e}")
            }
            BackendError::MapFailed(e) => write!(f, "map failed: {e}"),
            BackendError::SignalFailed(e) => write!(f, "fence signal failed: {e}"),
            BackendError::WaitFailed(msg) => write!(f, "fence wait failed: {msg}"),
            BackendError::ReservedTextureId(id) => write!(
                f,
                "texture id {id} is reserved for the built-in font atlas (FONT = 0, WHITE = 1)"
            ),
            BackendError::InvalidTextureData(msg) => write!(f, "invalid texture data: {msg}"),
            BackendError::DescriptorHeapExhausted(capacity) => {
                write!(f, "SRV descriptor heap exhausted ({capacity} slots)")
            }
        }
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BackendError::DeviceCreation(e)
            | BackendError::CommandQueueCreation(e)
            | BackendError::FenceCreation(e)
            | BackendError::EventCreation(e)
            | BackendError::CommandAllocatorCreation(e)
            | BackendError::CommandListCreation(e)
            | BackendError::CommandRecording(_, e)
            | BackendError::StateCreation(_, e)
            | BackendError::TextureCreation(e)
            | BackendError::BufferCreation(e)
            | BackendError::DescriptorHeapCreation(e)
            | BackendError::MapFailed(e)
            | BackendError::SignalFailed(e) => Some(e),
            BackendError::ShaderCompile(_)
            | BackendError::WaitFailed(_)
            | BackendError::ReservedTextureId(_)
            | BackendError::InvalidTextureData(_)
            | BackendError::DescriptorHeapExhausted(_) => None,
        }
    }
}

pub const FRAMES_IN_FLIGHT: usize = 2;

pub struct FrameSync {
    fence: ID3D12Fence,
    event: HANDLE,
    next_value: u64,
}

impl FrameSync {
    pub fn new(device: &Device) -> Result<Self, BackendError> {
        let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }
            .map_err(BackendError::FenceCreation)?;
        let event = unsafe { CreateEventW(None, false, false, None) }
            .map_err(BackendError::EventCreation)?;
        Ok(Self {
            fence,
            event,
            next_value: 1,
        })
    }

    pub fn signal(&mut self, queue: &Queue) -> Result<u64, BackendError> {
        let value = self.next_value;
        unsafe { queue.Signal(&self.fence, value) }.map_err(BackendError::SignalFailed)?;
        self.next_value += 1;
        Ok(value)
    }

    pub fn wait(&self, value: u64) -> Result<(), BackendError> {
        if self.completed_value() < value {
            unsafe { self.fence.SetEventOnCompletion(value, self.event) }
                .map_err(|e| BackendError::WaitFailed(format!("SetEventOnCompletion: {e}")))?;
            let result = unsafe { WaitForSingleObject(self.event, INFINITE) };
            if result != WAIT_OBJECT_0 {
                return Err(BackendError::WaitFailed(format!(
                    "WaitForSingleObject returned {result:?}"
                )));
            }
        }
        Ok(())
    }

    pub fn signal_and_wait(&mut self, queue: &Queue) -> Result<(), BackendError> {
        let value = self.signal(queue)?;
        self.wait(value)
    }

    pub fn completed_value(&self) -> u64 {
        unsafe { self.fence.GetCompletedValue() }
    }

    pub fn last_signaled_value(&self) -> u64 {
        self.next_value - 1
    }
}

impl Drop for FrameSync {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.event);
        }
    }
}

pub struct D3d12Backend {
    device: Device,
    queue: Queue,
    sync: FrameSync,
    pipeline: pipeline::Pipeline,
}

impl D3d12Backend {
    pub fn new(device: &Device, queue: &Queue) -> Result<Self, BackendError> {
        let mut sync = FrameSync::new(device)?;
        let pipeline = pipeline::Pipeline::new(device, queue, &mut sync)?;
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            sync,
            pipeline,
        })
    }

    pub fn create_device() -> Result<(Self, Device, Queue), BackendError> {
        let mut device: Option<Device> = None;
        unsafe { D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device) }
            .map_err(BackendError::DeviceCreation)?;
        let device = device.ok_or_else(|| {
            BackendError::DeviceCreation(windows::core::Error::from_hresult(E_UNEXPECTED))
        })?;

        let queue_desc = D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            Priority: 0,
            Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
            NodeMask: 0,
        };
        let queue: Queue = unsafe { device.CreateCommandQueue(&queue_desc) }
            .map_err(BackendError::CommandQueueCreation)?;

        let backend = Self::new(&device, &queue)?;
        Ok((backend, device, queue))
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn sync(&self) -> &FrameSync {
        &self.sync
    }

    pub fn sync_mut(&mut self) -> &mut FrameSync {
        &mut self.sync
    }

    pub fn upload_atlas(
        &mut self,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        self.pipeline
            .upload_atlas(&mut self.sync, pixels, width, height)
    }

    pub fn add_host_texture(
        &mut self,
        id: TextureId,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        self.pipeline
            .add_host_texture(&mut self.sync, id, rgba, width, height)
    }

    pub fn render(
        &mut self,
        draw_data: &DrawData,
        target: &ID3D12Resource,
        rtv: D3D12_CPU_DESCRIPTOR_HANDLE,
        viewport_size: [f32; 2],
    ) -> Result<(), BackendError> {
        let _ = target;
        self.pipeline
            .render(&mut self.sync, draw_data, rtv, viewport_size)
    }

    pub fn retirement_queue_len(&self) -> usize {
        self.pipeline.retirement_queue_len()
    }

    // A counter that exists purely so tests can prove we never stall the whole GPU in steady state. Paranoia, but make it observable.
    pub fn full_idle_wait_count(&self) -> u64 {
        self.pipeline.full_idle_wait_count()
    }
}

impl Drop for D3d12Backend {
    fn drop(&mut self) {
        let _ = self.sync.wait(self.sync.last_signaled_value());
    }
}

pub(crate) fn record_transition_barrier(
    list: &ID3D12GraphicsCommandList,
    resource: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) {
    use windows::Win32::Graphics::Direct3D12::{
        D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_FLAG_NONE,
        D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_TRANSITION_BARRIER,
    };
    let barrier = D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: std::mem::ManuallyDrop::new(Some(resource.clone())),
                Subresource: 0,
                StateBefore: before,
                StateAfter: after,
            }),
        },
    };
    unsafe {
        list.ResourceBarrier(std::slice::from_ref(&barrier));
        let transition = std::mem::ManuallyDrop::into_inner(barrier.Anonymous.Transition);
        drop(std::mem::ManuallyDrop::into_inner(transition.pResource));
    }
}

pub(crate) fn expect_resource(
    resource: Option<ID3D12Resource>,
    error: impl FnOnce(windows::core::Error) -> BackendError,
) -> Result<ID3D12Resource, BackendError> {
    resource.ok_or_else(|| {
        error(windows::core::Error::from_hresult(
            windows::Win32::Foundation::E_UNEXPECTED,
        ))
    })
}
