use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandAllocator, ID3D12CommandList, ID3D12DescriptorHeap, ID3D12GraphicsCommandList,
    ID3D12Resource, D3D12_CLEAR_VALUE, D3D12_CLEAR_VALUE_0, D3D12_COMMAND_LIST_TYPE_DIRECT,
    D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
    D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES,
    D3D12_HEAP_TYPE_DEFAULT, D3D12_HEAP_TYPE_READBACK, D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_DIMENSION_TEXTURE2D,
    D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET, D3D12_RESOURCE_FLAG_NONE,
    D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_COPY_SOURCE,
    D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
    D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
    D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_TEXTURE_LAYOUT_UNKNOWN,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};

use crate::{expect_resource, record_transition_barrier, BackendError, Device, FrameSync, Queue};

pub struct OffscreenTarget {
    texture: ID3D12Resource,
    _rtv_heap: ID3D12DescriptorHeap,
    rtv: D3D12_CPU_DESCRIPTOR_HANDLE,
    // The READBACK heap: the only D3D12 memory the CPU is allowed to peek at. Everything else is "GPU says no".
    readback: ID3D12Resource,
    readback_footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    allocator: ID3D12CommandAllocator,
    list: ID3D12GraphicsCommandList,
    width: u32,
    height: u32,
}

impl OffscreenTarget {
    pub fn new(device: &Device, width: u32, height: u32) -> Result<Self, BackendError> {
        let texture_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: width as u64,
            Height: height,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,
        };
        let default_heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            ..Default::default()
        };
        let clear_value = D3D12_CLEAR_VALUE {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            Anonymous: D3D12_CLEAR_VALUE_0 {
                Color: [0.0, 0.0, 0.0, 0.0],
            },
        };
        let mut texture: Option<ID3D12Resource> = None;
        unsafe {
            device.CreateCommittedResource(
                &default_heap,
                D3D12_HEAP_FLAG_NONE,
                &texture_desc,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                Some(&clear_value),
                &mut texture,
            )
        }
        .map_err(BackendError::TextureCreation)?;
        let texture = expect_resource(texture, BackendError::TextureCreation)?;

        let rtv_heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            NumDescriptors: 1,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 0,
        };
        let rtv_heap: ID3D12DescriptorHeap = unsafe { device.CreateDescriptorHeap(&rtv_heap_desc) }
            .map_err(BackendError::DescriptorHeapCreation)?;
        let rtv = unsafe { rtv_heap.GetCPUDescriptorHandleForHeapStart() };
        unsafe { device.CreateRenderTargetView(&texture, None, rtv) };

        let mut readback_footprint = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        let mut total_bytes = 0u64;
        unsafe {
            device.GetCopyableFootprints(
                &texture_desc,
                0,
                1,
                0,
                Some(&mut readback_footprint),
                None,
                None,
                Some(&mut total_bytes),
            );
        }
        let readback_heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_READBACK,
            ..Default::default()
        };
        let buffer_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: 0,
            Width: total_bytes,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };
        let mut readback: Option<ID3D12Resource> = None;
        unsafe {
            device.CreateCommittedResource(
                &readback_heap,
                D3D12_HEAP_FLAG_NONE,
                &buffer_desc,
                D3D12_RESOURCE_STATE_COPY_DEST,
                None,
                &mut readback,
            )
        }
        .map_err(BackendError::BufferCreation)?;
        let readback = expect_resource(readback, BackendError::BufferCreation)?;

        let allocator: ID3D12CommandAllocator =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }
                .map_err(BackendError::CommandAllocatorCreation)?;
        let list: ID3D12GraphicsCommandList = unsafe {
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
        }
        .map_err(BackendError::CommandListCreation)?;
        unsafe { list.Close() }
            .map_err(|e| BackendError::CommandRecording("command list close", e))?;

        Ok(Self {
            texture,
            _rtv_heap: rtv_heap,
            rtv,
            readback,
            readback_footprint,
            allocator,
            list,
            width,
            height,
        })
    }

    pub fn resource(&self) -> &ID3D12Resource {
        &self.texture
    }

    pub fn rtv(&self) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        self.rtv
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(
        &self,
        queue: &Queue,
        sync: &mut FrameSync,
        color: [f32; 4],
    ) -> Result<(), BackendError> {
        self.record(queue, |list| unsafe {
            list.ClearRenderTargetView(self.rtv, &color, None);
        })?;
        sync.signal_and_wait(queue)
    }

    pub fn readback(&self, queue: &Queue, sync: &mut FrameSync) -> Result<Vec<u8>, BackendError> {
        self.record(queue, |list| unsafe {
            record_transition_barrier(
                list,
                &self.texture,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_STATE_COPY_SOURCE,
            );
            let dst = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::ManuallyDrop::new(Some(self.readback.clone())),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: self.readback_footprint,
                },
            };
            let src = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::ManuallyDrop::new(Some(self.texture.clone())),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };
            list.CopyTextureRegion(&dst, 0, 0, 0, &src, None);
            drop(std::mem::ManuallyDrop::into_inner(dst.pResource));
            drop(std::mem::ManuallyDrop::into_inner(src.pResource));
            record_transition_barrier(
                list,
                &self.texture,
                D3D12_RESOURCE_STATE_COPY_SOURCE,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
            );
        })?;
        sync.signal_and_wait(queue)?;

        let row_bytes = self.width as usize * 4;
        let mut out = vec![0u8; row_bytes * self.height as usize];
        unsafe {
            let mut data = std::ptr::null_mut();
            self.readback
                .Map(0, None, Some(&mut data))
                .map_err(BackendError::MapFailed)?;
            if data.is_null() {
                self.readback.Unmap(0, None);
                return Err(BackendError::MapFailed(windows::core::Error::from_hresult(
                    windows::Win32::Foundation::E_UNEXPECTED,
                )));
            }
            let src = data as *const u8;
            let row_pitch = self.readback_footprint.Footprint.RowPitch as usize;
            for row in 0..self.height as usize {
                let src_row = src.add(row * row_pitch);
                let dst_row = out.as_mut_ptr().add(row * row_bytes);
                std::ptr::copy_nonoverlapping(src_row, dst_row, row_bytes);
            }
            self.readback.Unmap(0, None);
        }
        Ok(out)
    }

    fn record(
        &self,
        queue: &Queue,
        f: impl FnOnce(&ID3D12GraphicsCommandList),
    ) -> Result<(), BackendError> {
        unsafe {
            self.allocator
                .Reset()
                .map_err(|e| BackendError::CommandRecording("command allocator reset", e))?;
            self.list
                .Reset(&self.allocator, None)
                .map_err(|e| BackendError::CommandRecording("command list reset", e))?;
            f(&self.list);
            self.list
                .Close()
                .map_err(|e| BackendError::CommandRecording("command list close", e))?;
            let list: ID3D12CommandList = self.list.clone().into();
            queue.ExecuteCommandLists(&[Some(list)]);
        }
        Ok(())
    }
}
