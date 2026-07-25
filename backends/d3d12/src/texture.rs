use std::collections::{HashMap, VecDeque};

use restricted_gui::draw::TextureId;
use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandAllocator, ID3D12CommandList, ID3D12DescriptorHeap, ID3D12GraphicsCommandList,
    ID3D12Resource, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_CPU_DESCRIPTOR_HANDLE,
    D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING, D3D12_DESCRIPTOR_HEAP_DESC,
    D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
    D3D12_GPU_DESCRIPTOR_HANDLE, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES,
    D3D12_HEAP_TYPE_DEFAULT, D3D12_PLACED_SUBRESOURCE_FOOTPRINT, D3D12_RESOURCE_DESC,
    D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_COPY_DEST,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_SHADER_RESOURCE_VIEW_DESC,
    D3D12_SHADER_RESOURCE_VIEW_DESC_0, D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_SUBRESOURCE_FOOTPRINT,
    D3D12_TEX2D_SRV, D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
    D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
    D3D12_TEXTURE_LAYOUT_UNKNOWN,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

use crate::pipeline::{Retired, RetiredKind, UploadRing};
use crate::{expect_resource, record_transition_barrier, BackendError, Device, FrameSync, Queue};

const SRV_HEAP_CAPACITY: u32 = 64;

const FIRST_HOST_SLOT: u32 = 1;

const PITCH_ALIGNMENT: u64 = 256;

struct Texture {
    resource: ID3D12Resource,
    width: u32,
    height: u32,
}

struct HostTexture {
    _texture: Texture,
    slot: u32,
}

pub(crate) struct UploadContext<'a> {
    pub(crate) device: &'a Device,
    pub(crate) queue: &'a Queue,
    pub(crate) sync: &'a mut FrameSync,
    pub(crate) retired: &'a mut VecDeque<Retired>,
}

impl UploadContext<'_> {
    fn retire_resource(&mut self, resource: ID3D12Resource) {
        self.retired.push_back(Retired {
            fence: self.sync.last_signaled_value(),
            kind: RetiredKind::Resource(resource),
        });
    }

    fn retire_recording(
        &mut self,
        allocator: ID3D12CommandAllocator,
        list: ID3D12GraphicsCommandList,
    ) {
        self.retired.push_back(Retired {
            fence: self.sync.last_signaled_value(),
            // Every upload gets a fresh one-shot allocator + command list, retired with its staging buffer. Uploads never wait at all -- correctness comes from the queue being polite (in-order execution).
            kind: RetiredKind::CommandRecording(allocator, list),
        });
    }

    fn retire_texture_slot(&mut self, resource: ID3D12Resource, slot: u32) {
        self.retired.push_back(Retired {
            fence: self.sync.last_signaled_value(),
            kind: RetiredKind::TextureSlot(resource, slot),
        });
    }
}

pub(crate) struct TextureSet {
    heap: ID3D12DescriptorHeap,
    descriptor_size: u32,
    cpu_start: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_start: D3D12_GPU_DESCRIPTOR_HANDLE,
    atlas: Texture,
    atlas_slot: u32,
    host: HashMap<u64, HostTexture>,
    free_slots: Vec<u32>,
    next_fresh_slot: u32,
}

impl TextureSet {
    pub(crate) fn new(ctx: &mut UploadContext) -> Result<Self, BackendError> {
        let desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            NumDescriptors: SRV_HEAP_CAPACITY,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            NodeMask: 0,
        };
        let heap: ID3D12DescriptorHeap = unsafe { ctx.device.CreateDescriptorHeap(&desc) }
            .map_err(BackendError::DescriptorHeapCreation)?;
        let descriptor_size = unsafe {
            ctx.device
                .GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV)
        };
        let cpu_start = unsafe { heap.GetCPUDescriptorHandleForHeapStart() };
        let gpu_start = unsafe { heap.GetGPUDescriptorHandleForHeapStart() };

        let white = [255u8, 255, 255, 255];
        let atlas = upload_texture(ctx, None, &white, 1, 1)?;
        let set = Self {
            heap,
            descriptor_size,
            cpu_start,
            gpu_start,
            atlas,
            atlas_slot: 0,
            host: HashMap::new(),
            free_slots: Vec::new(),
            next_fresh_slot: FIRST_HOST_SLOT,
        };
        set.create_srv(ctx.device, &set.atlas.resource, set.atlas_slot);
        Ok(set)
    }

    pub(crate) fn heap(&self) -> &ID3D12DescriptorHeap {
        &self.heap
    }

    fn take_slot(&mut self) -> Result<u32, BackendError> {
        if let Some(slot) = self.free_slots.pop() {
            return Ok(slot);
        }
        if self.next_fresh_slot >= SRV_HEAP_CAPACITY {
            return Err(BackendError::DescriptorHeapExhausted(SRV_HEAP_CAPACITY));
        }
        let slot = self.next_fresh_slot;
        self.next_fresh_slot += 1;
        Ok(slot)
    }

    pub(crate) fn return_slot(&mut self, slot: u32) {
        self.free_slots.push(slot);
    }

    pub(crate) fn srv_for(&self, id: TextureId) -> Option<D3D12_GPU_DESCRIPTOR_HANDLE> {
        match id {
            TextureId::FONT | TextureId::WHITE => Some(self.gpu_handle(self.atlas_slot)),
            _ => self.host.get(&id.0).map(|t| self.gpu_handle(t.slot)),
        }
    }

    pub(crate) fn upload_atlas(
        &mut self,
        ctx: &mut UploadContext,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        let expected = width as usize * height as usize;
        if width == 0 || height == 0 || pixels.len() != expected {
            return Err(BackendError::InvalidTextureData(format!(
                "atlas pixel data is {} bytes, expected width * height = \
                 {width} * {height} = {expected}",
                pixels.len()
            )));
        }
        let mut rgba = Vec::with_capacity(expected * 4);
        for &a in pixels {
            rgba.extend_from_slice(&[255, 255, 255, a]);
        }
        if self.atlas.width == width && self.atlas.height == height {
            self.atlas = upload_texture(ctx, Some(&self.atlas), &rgba, width, height)?;
            return Ok(());
        }
        let slot = self.take_slot()?;
        let atlas = match upload_texture(ctx, None, &rgba, width, height) {
            Ok(atlas) => atlas,
            Err(e) => {
                self.free_slots.push(slot);
                return Err(e);
            }
        };
        self.create_srv(ctx.device, &atlas.resource, slot);
        let old = std::mem::replace(&mut self.atlas, atlas);
        let old_slot = std::mem::replace(&mut self.atlas_slot, slot);
        ctx.retire_texture_slot(old.resource, old_slot);
        Ok(())
    }

    pub(crate) fn add_host_texture(
        &mut self,
        ctx: &mut UploadContext,
        id: TextureId,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        if id == TextureId::FONT || id == TextureId::WHITE {
            return Err(BackendError::ReservedTextureId(id.0));
        }
        let expected = width as usize * height as usize * 4;
        if width == 0 || height == 0 || rgba.len() != expected {
            return Err(BackendError::InvalidTextureData(format!(
                "host texture data is {} bytes, expected width * height * 4 = \
                 {width} * {height} * 4 = {expected}",
                rgba.len()
            )));
        }
        let slot = self.take_slot()?;
        let texture = match upload_texture(ctx, None, rgba, width, height) {
            Ok(texture) => texture,
            Err(e) => {
                self.free_slots.push(slot);
                return Err(e);
            }
        };
        self.create_srv(ctx.device, &texture.resource, slot);
        if let Some(old) = self.host.insert(
            id.0,
            HostTexture {
                _texture: texture,
                slot,
            },
        ) {
            ctx.retire_texture_slot(old._texture.resource, old.slot);
        }
        Ok(())
    }

    fn create_srv(&self, device: &Device, resource: &ID3D12Resource, slot: u32) {
        let desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D12_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                    PlaneSlice: 0,
                    ResourceMinLODClamp: 0.0,
                },
            },
        };
        unsafe { device.CreateShaderResourceView(resource, Some(&desc), self.cpu_handle(slot)) };
    }

    fn cpu_handle(&self, slot: u32) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        D3D12_CPU_DESCRIPTOR_HANDLE {
            ptr: self.cpu_start.ptr + (slot * self.descriptor_size) as usize,
        }
    }

    fn gpu_handle(&self, slot: u32) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        D3D12_GPU_DESCRIPTOR_HANDLE {
            ptr: self.gpu_start.ptr + (slot * self.descriptor_size) as u64,
        }
    }
}

fn upload_texture(
    ctx: &mut UploadContext,
    existing: Option<&Texture>,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<Texture, BackendError> {
    let texture = match existing {
        Some(texture) => texture.resource.clone(),
        None => create_texture_resource(ctx.device, width, height)?,
    };

    let row_bytes = width as usize * 4;
    let row_pitch = align_up(row_bytes as u64, PITCH_ALIGNMENT) as usize;
    let staging = UploadRing::new(ctx.device, (row_pitch * height as usize) as u64)?;

    let allocator: ID3D12CommandAllocator = unsafe {
        ctx.device
            .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
    }
    .map_err(BackendError::CommandAllocatorCreation)?;
    let list: ID3D12GraphicsCommandList = unsafe {
        ctx.device
            .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
    }
    .map_err(BackendError::CommandListCreation)?;

    unsafe {
        for row in 0..height as usize {
            std::ptr::copy_nonoverlapping(
                rgba.as_ptr().add(row * row_bytes),
                staging.mapped.add(row * row_pitch),
                row_bytes,
            );
        }

        if existing.is_some() {
            record_transition_barrier(
                &list,
                &texture,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                D3D12_RESOURCE_STATE_COPY_DEST,
            );
        }
        let dst = D3D12_TEXTURE_COPY_LOCATION {
            pResource: std::mem::ManuallyDrop::new(Some(texture.clone())),
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                SubresourceIndex: 0,
            },
        };
        let src = D3D12_TEXTURE_COPY_LOCATION {
            pResource: std::mem::ManuallyDrop::new(Some(staging.buffer.clone())),
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                    Offset: 0,
                    Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                        Width: width,
                        Height: height,
                        Depth: 1,
                        RowPitch: row_pitch as u32,
                    },
                },
            },
        };
        list.CopyTextureRegion(&dst, 0, 0, 0, &src, None);
        drop(std::mem::ManuallyDrop::into_inner(dst.pResource));
        drop(std::mem::ManuallyDrop::into_inner(src.pResource));

        record_transition_barrier(
            &list,
            &texture,
            D3D12_RESOURCE_STATE_COPY_DEST,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        );
        list.Close()
            .map_err(|e| BackendError::CommandRecording("command list close", e))?;
        let command_list: ID3D12CommandList = list.clone().into();
        ctx.queue.ExecuteCommandLists(&[Some(command_list)]);
    }
    let value = ctx.sync.signal(ctx.queue)?;
    debug_assert_eq!(value, ctx.sync.last_signaled_value());
    ctx.retire_resource(staging.buffer);
    ctx.retire_recording(allocator, list);
    Ok(Texture {
        resource: texture,
        width,
        height,
    })
}

fn create_texture_resource(
    device: &Device,
    width: u32,
    height: u32,
) -> Result<ID3D12Resource, BackendError> {
    let desc = D3D12_RESOURCE_DESC {
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
        Flags: D3D12_RESOURCE_FLAG_NONE,
    };
    let heap = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_DEFAULT,
        ..Default::default()
    };
    let mut texture: Option<ID3D12Resource> = None;
    unsafe {
        device.CreateCommittedResource(
            &heap,
            D3D12_HEAP_FLAG_NONE,
            &desc,
            D3D12_RESOURCE_STATE_COPY_DEST,
            None,
            &mut texture,
        )
    }
    .map_err(BackendError::TextureCreation)?;
    expect_resource(texture, BackendError::TextureCreation)
}

fn align_up(value: u64, alignment: u64) -> u64 {
    (value + alignment - 1) & !(alignment - 1)
}
