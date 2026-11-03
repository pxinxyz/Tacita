use crate::{expect_resource, BackendError, Device, Queue};
use windows::Win32::Graphics::Direct3D12::*;
pub(crate) struct GpuTimer {
    heap: ID3D12QueryHeap,
    readback: ID3D12Resource,
    frequency: u64,
}
impl GpuTimer {
    pub fn new(device: &Device, queue: &Queue) -> Result<Self, BackendError> {
        let mut heap: Option<ID3D12QueryHeap> = None;
        unsafe {
            device.CreateQueryHeap(
                &D3D12_QUERY_HEAP_DESC {
                    Type: D3D12_QUERY_HEAP_TYPE_TIMESTAMP,
                    Count: 2,
                    NodeMask: 0,
                },
                &mut heap,
            )
        }
        .map_err(|e| BackendError::StateCreation("timestamp heap", e))?;
        let heap =
            heap.ok_or_else(|| BackendError::InvalidTextureData("missing timestamp heap".into()))?;
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: 16,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };
        let props = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_READBACK,
            ..Default::default()
        };
        let mut readback = None;
        unsafe {
            device.CreateCommittedResource(
                &props,
                D3D12_HEAP_FLAG_NONE,
                &desc,
                D3D12_RESOURCE_STATE_COPY_DEST,
                None,
                &mut readback,
            )
        }
        .map_err(BackendError::BufferCreation)?;
        let frequency = unsafe { queue.GetTimestampFrequency() }
            .map_err(|e| BackendError::StateCreation("timestamp frequency", e))?;
        Ok(Self {
            heap,
            readback: expect_resource(readback, BackendError::BufferCreation)?,
            frequency,
        })
    }
    pub fn begin(&self, list: &ID3D12GraphicsCommandList) {
        unsafe {
            list.EndQuery(&self.heap, D3D12_QUERY_TYPE_TIMESTAMP, 0);
        }
    }
    pub fn end(&self, list: &ID3D12GraphicsCommandList) {
        unsafe {
            list.EndQuery(&self.heap, D3D12_QUERY_TYPE_TIMESTAMP, 1);
            list.ResolveQueryData(
                &self.heap,
                D3D12_QUERY_TYPE_TIMESTAMP,
                0,
                2,
                &self.readback,
                0,
            );
        }
    }
    pub fn read(&self) -> Result<Option<f64>, BackendError> {
        let mut pointer = core::ptr::null_mut();
        unsafe {
            self.readback.Map(
                0,
                Some(&D3D12_RANGE { Begin: 0, End: 16 }),
                Some(&mut pointer),
            )
        }
        .map_err(BackendError::MapFailed)?;
        if pointer.is_null() {
            return Err(BackendError::InvalidTextureData(
                "null timestamp mapping".into(),
            ));
        }
        let (start, end) = unsafe {
            (
                core::ptr::read_unaligned(pointer.cast::<u64>()),
                core::ptr::read_unaligned(pointer.cast::<u64>().add(1)),
            )
        };
        unsafe {
            self.readback
                .Unmap(0, Some(&D3D12_RANGE { Begin: 0, End: 0 }));
        }
        Ok((end >= start && self.frequency > 0)
            .then(|| (end - start) as f64 * 1000. / self.frequency as f64))
    }
}
