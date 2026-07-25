use restricted_gui::draw::{DrawData, DrawVert, TextureId};
use windows::core::PCSTR;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
use windows::Win32::Graphics::Direct3D12::{
    D3D12SerializeRootSignature, ID3D12CommandAllocator, ID3D12CommandList,
    ID3D12GraphicsCommandList, ID3D12PipelineState, ID3D12Resource, ID3D12RootSignature,
    D3D12_BLEND_DESC, D3D12_BLEND_INV_SRC_ALPHA, D3D12_BLEND_ONE, D3D12_BLEND_OP_ADD,
    D3D12_BLEND_SRC_ALPHA, D3D12_COLOR_WRITE_ENABLE_ALL, D3D12_COMMAND_LIST_TYPE_DIRECT,
    D3D12_COMPARISON_FUNC_ALWAYS, D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
    D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_CULL_MODE_NONE, D3D12_DEPTH_STENCIL_DESC,
    D3D12_DEPTH_WRITE_MASK_ZERO, D3D12_DESCRIPTOR_RANGE, D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
    D3D12_FILL_MODE_SOLID, D3D12_FILTER_MIN_MAG_MIP_POINT, D3D12_GRAPHICS_PIPELINE_STATE_DESC,
    D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_UPLOAD, D3D12_INDEX_BUFFER_VIEW,
    D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, D3D12_INPUT_ELEMENT_DESC, D3D12_INPUT_LAYOUT_DESC,
    D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE, D3D12_RASTERIZER_DESC, D3D12_RENDER_TARGET_BLEND_DESC,
    D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_FLAG_NONE,
    D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_ROOT_DESCRIPTOR, D3D12_ROOT_DESCRIPTOR_TABLE,
    D3D12_ROOT_PARAMETER, D3D12_ROOT_PARAMETER_0, D3D12_ROOT_PARAMETER_TYPE_CBV,
    D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, D3D12_ROOT_SIGNATURE_DESC,
    D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT, D3D12_SHADER_BYTECODE,
    D3D12_SHADER_VISIBILITY_PIXEL, D3D12_SHADER_VISIBILITY_VERTEX,
    D3D12_STATIC_BORDER_COLOR_OPAQUE_WHITE, D3D12_STATIC_SAMPLER_DESC,
    D3D12_TEXTURE_ADDRESS_MODE_CLAMP, D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_VERTEX_BUFFER_VIEW,
    D3D12_VIEWPORT, D3D_ROOT_SIGNATURE_VERSION_1,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32_UINT, DXGI_FORMAT_R8G8B8A8_UNORM,
    DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};

use crate::shader;
use crate::texture::{TextureSet, UploadContext};
use crate::{expect_resource, BackendError, Device, FrameSync, Queue, FRAMES_IN_FLIGHT};

const VERTEX_STRIDE: u32 = 20;

const RING_INITIAL_CAPACITY: u64 = 4 * 1024 * 1024;

const RING_ALIGNMENT: u64 = 256;

const INPUT_LAYOUT: [D3D12_INPUT_ELEMENT_DESC; 3] = [
    D3D12_INPUT_ELEMENT_DESC {
        SemanticName: PCSTR::from_raw(c"POSITION".as_ptr().cast()),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 0,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D12_INPUT_ELEMENT_DESC {
        SemanticName: PCSTR::from_raw(c"TEXCOORD".as_ptr().cast()),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 8,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D12_INPUT_ELEMENT_DESC {
        SemanticName: PCSTR::from_raw(c"COLOR".as_ptr().cast()),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        InputSlot: 0,
        AlignedByteOffset: 16,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
];

pub(crate) struct UploadRing {
    pub(crate) buffer: ID3D12Resource,
    pub(crate) mapped: *mut u8,
    gpu_base: u64,
    capacity: u64,
}

impl UploadRing {
    pub(crate) fn new(device: &Device, capacity: u64) -> Result<Self, BackendError> {
        let heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_UPLOAD,
            ..Default::default()
        };
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: 0,
            Width: capacity,
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
        let mut buffer: Option<ID3D12Resource> = None;
        unsafe {
            device.CreateCommittedResource(
                &heap,
                D3D12_HEAP_FLAG_NONE,
                &desc,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &mut buffer,
            )
        }
        .map_err(BackendError::BufferCreation)?;
        let buffer = expect_resource(buffer, BackendError::BufferCreation)?;

        let mut data = std::ptr::null_mut();
        unsafe { buffer.Map(0, None, Some(&mut data)) }.map_err(BackendError::MapFailed)?;
        if data.is_null() {
            return Err(BackendError::MapFailed(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_UNEXPECTED,
            )));
        }
        let gpu_base = unsafe { buffer.GetGPUVirtualAddress() };
        Ok(Self {
            buffer,
            mapped: data.cast::<u8>(),
            gpu_base,
            capacity,
        })
    }

    fn ensure_capacity(
        &mut self,
        device: &Device,
        needed: u64,
    ) -> Result<Option<UploadRing>, BackendError> {
        if needed <= self.capacity {
            return Ok(None);
        }
        let mut capacity = self.capacity.max(1);
        while capacity < needed {
            capacity *= 2;
        }
        Ok(Some(std::mem::replace(self, Self::new(device, capacity)?)))
    }
}

pub(crate) struct Retired {
    pub(crate) fence: u64,
    pub(crate) kind: RetiredKind,
}

// allow(dead_code) stays: the payloads are never READ, they exist so their COM refs stay alive until the fence passes. We tried removing this attribute once. The compiler tattled.
#[allow(dead_code)]
pub(crate) enum RetiredKind {
    Resource(ID3D12Resource),
    CommandRecording(ID3D12CommandAllocator, ID3D12GraphicsCommandList),
    TextureSlot(ID3D12Resource, u32),
}

struct FrameSlot {
    allocator: ID3D12CommandAllocator,
    list: ID3D12GraphicsCommandList,
    ring: UploadRing,
    last_signaled: Option<u64>,
}

impl FrameSlot {
    fn new(device: &Device) -> Result<Self, BackendError> {
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
            allocator,
            list,
            ring: UploadRing::new(device, RING_INITIAL_CAPACITY)?,
            last_signaled: None,
        })
    }
}

pub(crate) struct Pipeline {
    device: Device,
    queue: Queue,
    root_signature: ID3D12RootSignature,
    pso: ID3D12PipelineState,
    textures: TextureSet,
    slots: Vec<FrameSlot>,
    current_slot: usize,
    // Deferred release queue: replaced resources wait here until their fence passes. Remove this and the GPU eats a texture mid-frame.
    retired: std::collections::VecDeque<Retired>,
    full_idle_waits: u64,
}

impl Pipeline {
    pub(crate) fn new(
        device: &Device,
        queue: &Queue,
        sync: &mut FrameSync,
    ) -> Result<Self, BackendError> {
        let root_signature = create_root_signature(device)?;
        let pso = create_pso(device, &root_signature)?;

        let mut retired = std::collections::VecDeque::new();
        let textures = {
            let mut ctx = UploadContext {
                device,
                queue,
                sync,
                retired: &mut retired,
            };
            TextureSet::new(&mut ctx)?
        };

        let mut slots = Vec::with_capacity(FRAMES_IN_FLIGHT);
        for _ in 0..FRAMES_IN_FLIGHT {
            slots.push(FrameSlot::new(device)?);
        }

        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            root_signature,
            pso,
            textures,
            slots,
            current_slot: 0,
            retired,
            full_idle_waits: 0,
        })
    }

    pub(crate) fn upload_atlas(
        &mut self,
        sync: &mut FrameSync,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        self.sweep_retirements(sync);
        let mut ctx = UploadContext {
            device: &self.device,
            queue: &self.queue,
            sync,
            retired: &mut self.retired,
        };
        self.textures.upload_atlas(&mut ctx, pixels, width, height)
    }

    pub(crate) fn add_host_texture(
        &mut self,
        sync: &mut FrameSync,
        id: TextureId,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        self.sweep_retirements(sync);
        let mut ctx = UploadContext {
            device: &self.device,
            queue: &self.queue,
            sync,
            retired: &mut self.retired,
        };
        self.textures
            .add_host_texture(&mut ctx, id, rgba, width, height)
    }

    fn sweep_retirements(&mut self, sync: &FrameSync) {
        let completed = sync.completed_value();
        while self
            .retired
            .front()
            .is_some_and(|entry| entry.fence <= completed)
        {
            if let Some(entry) = self.retired.pop_front() {
                if let RetiredKind::TextureSlot(_, slot) = entry.kind {
                    self.textures.return_slot(slot);
                }
            }
        }
    }

    fn wait_tracked(&mut self, sync: &FrameSync, value: u64) -> Result<(), BackendError> {
        if value == sync.last_signaled_value() {
            self.full_idle_waits += 1;
        }
        sync.wait(value)
    }

    pub(crate) fn retirement_queue_len(&self) -> usize {
        self.retired.len()
    }

    pub(crate) fn full_idle_wait_count(&self) -> u64 {
        self.full_idle_waits
    }

    pub(crate) fn render(
        &mut self,
        sync: &mut FrameSync,
        draw_data: &DrawData,
        rtv: D3D12_CPU_DESCRIPTOR_HANDLE,
        viewport_size: [f32; 2],
    ) -> Result<(), BackendError> {
        self.sweep_retirements(sync);

        let [width, height] = viewport_size;
        if width <= 0.0
            || height <= 0.0
            || draw_data.vertices.is_empty()
            || draw_data.indices.is_empty()
        {
            return Ok(());
        }

        let slot_index = self.current_slot;
        if let Some(value) = self.slots[slot_index].last_signaled {
            self.wait_tracked(sync, value)?;
        }
        let slot = &mut self.slots[slot_index];

        let vertex_bytes = pack_vertices(draw_data.vertices);
        let index_bytes = pack_indices(draw_data.indices);
        let cb_offset = 0u64;
        let vb_offset = RING_ALIGNMENT;
        let ib_offset = align_up(vb_offset + vertex_bytes.len() as u64, RING_ALIGNMENT);
        if let Some(old_ring) = slot
            .ring
            .ensure_capacity(&self.device, ib_offset + index_bytes.len() as u64)?
        {
            self.retired.push_back(Retired {
                fence: sync.last_signaled_value(),
                kind: RetiredKind::Resource(old_ring.buffer),
            });
        }

        let projection_columns: [f32; 16] = [
            2.0 / width,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / height,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            -1.0,
            1.0,
            0.0,
            1.0,
        ];
        unsafe {
            let base = slot.ring.mapped;
            std::ptr::copy_nonoverlapping(
                projection_columns.as_ptr().cast::<u8>(),
                base.add(cb_offset as usize),
                64,
            );
            std::ptr::copy_nonoverlapping(
                vertex_bytes.as_ptr(),
                base.add(vb_offset as usize),
                vertex_bytes.len(),
            );
            std::ptr::copy_nonoverlapping(
                index_bytes.as_ptr(),
                base.add(ib_offset as usize),
                index_bytes.len(),
            );
        }

        unsafe {
            slot.allocator
                .Reset()
                .map_err(|e| BackendError::CommandRecording("command allocator reset", e))?;
            slot.list
                .Reset(&slot.allocator, None)
                .map_err(|e| BackendError::CommandRecording("command list reset", e))?;

            slot.list.SetPipelineState(&self.pso);
            slot.list.SetGraphicsRootSignature(&self.root_signature);
            slot.list
                .SetDescriptorHeaps(&[Some(self.textures.heap().clone())]);
            slot.list.OMSetRenderTargets(1, Some(&rtv), false, None);
            slot.list.RSSetViewports(&[D3D12_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: width,
                Height: height,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]);
            slot.list
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            let vertex_view = D3D12_VERTEX_BUFFER_VIEW {
                BufferLocation: slot.ring.gpu_base + vb_offset,
                SizeInBytes: vertex_bytes.len() as u32,
                StrideInBytes: VERTEX_STRIDE,
            };
            slot.list.IASetVertexBuffers(0, Some(&[vertex_view]));
            let index_view = D3D12_INDEX_BUFFER_VIEW {
                BufferLocation: slot.ring.gpu_base + ib_offset,
                SizeInBytes: index_bytes.len() as u32,
                Format: DXGI_FORMAT_R32_UINT,
            };
            slot.list.IASetIndexBuffer(Some(&index_view));
            slot.list
                .SetGraphicsRootConstantBufferView(0, slot.ring.gpu_base + cb_offset);

            let mut index_offset: u32 = 0;
            for cmd in draw_data.commands {
                let start_index = index_offset;
                index_offset += cmd.elem_count;
                if cmd.elem_count == 0 {
                    continue;
                }
                let Some(scissor) = scissor_rect(cmd.clip_rect, width, height) else {
                    continue;
                };
                let Some(srv) = self.textures.srv_for(cmd.texture) else {
                    eprintln!(
                        "restricted-gui-d3d12: skipping draw command: texture id {} is not \
                         registered (backend contract §7)",
                        cmd.texture.0
                    );
                    continue;
                };
                slot.list.RSSetScissorRects(&[scissor]);
                slot.list.SetGraphicsRootDescriptorTable(1, srv);
                slot.list
                    .DrawIndexedInstanced(cmd.elem_count, 1, start_index, 0, 0);
            }

            slot.list
                .Close()
                .map_err(|e| BackendError::CommandRecording("command list close", e))?;
            let list: ID3D12CommandList = slot.list.clone().into();
            self.queue.ExecuteCommandLists(&[Some(list)]);
        }
        slot.last_signaled = Some(sync.signal(&self.queue)?);
        self.current_slot = (slot_index + 1) % self.slots.len();
        Ok(())
    }
}

fn create_root_signature(device: &Device) -> Result<ID3D12RootSignature, BackendError> {
    let srv_range = D3D12_DESCRIPTOR_RANGE {
        RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
        NumDescriptors: 1,
        BaseShaderRegister: 0,
        RegisterSpace: 0,
        OffsetInDescriptorsFromTableStart: 0,
    };
    let parameters = [
        D3D12_ROOT_PARAMETER {
            ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
            Anonymous: D3D12_ROOT_PARAMETER_0 {
                Descriptor: D3D12_ROOT_DESCRIPTOR {
                    ShaderRegister: 0,
                    RegisterSpace: 0,
                },
            },
            ShaderVisibility: D3D12_SHADER_VISIBILITY_VERTEX,
        },
        D3D12_ROOT_PARAMETER {
            ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
            Anonymous: D3D12_ROOT_PARAMETER_0 {
                DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                    NumDescriptorRanges: 1,
                    pDescriptorRanges: &srv_range,
                },
            },
            ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
        },
    ];
    let sampler = D3D12_STATIC_SAMPLER_DESC {
        Filter: D3D12_FILTER_MIN_MAG_MIP_POINT,
        AddressU: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
        AddressV: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
        AddressW: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
        MipLODBias: 0.0,
        MaxAnisotropy: 0,
        ComparisonFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        BorderColor: D3D12_STATIC_BORDER_COLOR_OPAQUE_WHITE,
        MinLOD: 0.0,
        MaxLOD: 0.0,
        ShaderRegister: 0,
        RegisterSpace: 0,
        ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
    };
    let desc = D3D12_ROOT_SIGNATURE_DESC {
        NumParameters: parameters.len() as u32,
        pParameters: parameters.as_ptr(),
        NumStaticSamplers: 1,
        pStaticSamplers: &sampler,
        Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
    };

    let mut blob = None;
    let mut errors = None;
    let result = unsafe {
        D3D12SerializeRootSignature(
            &desc,
            D3D_ROOT_SIGNATURE_VERSION_1,
            &mut blob,
            Some(&mut errors),
        )
    };
    let blob = match (result, blob) {
        (Ok(()), Some(blob)) => blob,
        (Ok(()), None) => {
            return Err(BackendError::ShaderCompile(
                "D3D12SerializeRootSignature succeeded without a blob".to_string(),
            ))
        }
        (Err(e), _) => {
            let detail = shader::error_blob_text(errors);
            if detail.is_empty() {
                return Err(BackendError::StateCreation("root signature", e));
            }
            return Err(BackendError::ShaderCompile(detail));
        }
    };
    let bytes = unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize())
    };
    unsafe { device.CreateRootSignature(0, bytes) }
        .map_err(|e| BackendError::StateCreation("root signature", e))
}

fn create_pso(
    device: &Device,
    root_signature: &ID3D12RootSignature,
) -> Result<ID3D12PipelineState, BackendError> {
    let vs_blob = shader::compile(
        shader::VERTEX_SHADER_SOURCE,
        shader::VS_ENTRY,
        shader::VS_TARGET,
    )?;
    let ps_blob = shader::compile(
        shader::PIXEL_SHADER_SOURCE,
        shader::PS_ENTRY,
        shader::PS_TARGET,
    )?;
    let bytecode = |blob: &windows::Win32::Graphics::Direct3D::ID3DBlob| D3D12_SHADER_BYTECODE {
        pShaderBytecode: unsafe { blob.GetBufferPointer() },
        BytecodeLength: unsafe { blob.GetBufferSize() },
    };

    let mut blend_desc = D3D12_BLEND_DESC::default();
    blend_desc.RenderTarget[0] = D3D12_RENDER_TARGET_BLEND_DESC {
        BlendEnable: true.into(),
        SrcBlend: D3D12_BLEND_SRC_ALPHA,
        DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
        BlendOp: D3D12_BLEND_OP_ADD,
        SrcBlendAlpha: D3D12_BLEND_ONE,
        DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
        BlendOpAlpha: D3D12_BLEND_OP_ADD,
        RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
        ..Default::default()
    };

    let rasterizer_desc = D3D12_RASTERIZER_DESC {
        FillMode: D3D12_FILL_MODE_SOLID,
        CullMode: D3D12_CULL_MODE_NONE,
        DepthClipEnable: true.into(),
        ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
        ..Default::default()
    };

    let depth_stencil_desc = D3D12_DEPTH_STENCIL_DESC {
        DepthEnable: false.into(),
        DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
        DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        StencilEnable: false.into(),
        ..Default::default()
    };

    let mut rtv_formats = [DXGI_FORMAT_UNKNOWN; 8];
    rtv_formats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;

    let pso_desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
        pRootSignature: std::mem::ManuallyDrop::new(Some(root_signature.clone())),
        VS: bytecode(&vs_blob),
        PS: bytecode(&ps_blob),
        BlendState: blend_desc,
        SampleMask: u32::MAX,
        RasterizerState: rasterizer_desc,
        DepthStencilState: depth_stencil_desc,
        InputLayout: D3D12_INPUT_LAYOUT_DESC {
            pInputElementDescs: INPUT_LAYOUT.as_ptr(),
            NumElements: INPUT_LAYOUT.len() as u32,
        },
        PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        NumRenderTargets: 1,
        RTVFormats: rtv_formats,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        ..Default::default()
    };
    let result: windows::core::Result<ID3D12PipelineState> =
        unsafe { device.CreateGraphicsPipelineState(&pso_desc) };
    drop(std::mem::ManuallyDrop::into_inner(pso_desc.pRootSignature));
    result.map_err(|e| BackendError::StateCreation("pipeline state", e))
}

fn align_up(value: u64, alignment: u64) -> u64 {
    (value + alignment - 1) & !(alignment - 1)
}

fn scissor_rect(clip_rect: [f32; 4], width: f32, height: f32) -> Option<RECT> {
    let min_x = clip_rect[0].clamp(0.0, width);
    let min_y = clip_rect[1].clamp(0.0, height);
    let max_x = clip_rect[2].clamp(0.0, width);
    let max_y = clip_rect[3].clamp(0.0, height);
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some(RECT {
        left: min_x.floor() as i32,
        top: min_y.floor() as i32,
        right: max_x.ceil() as i32,
        bottom: max_y.ceil() as i32,
    })
}

fn pack_vertices(vertices: &[DrawVert]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vertices.len() * VERTEX_STRIDE as usize);
    for v in vertices {
        for f in [v.pos[0], v.pos[1], v.uv[0], v.uv[1]] {
            out.extend_from_slice(&f.to_ne_bytes());
        }
        out.extend_from_slice(&v.col);
    }
    out
}

fn pack_indices(indices: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(indices.len() * 4);
    for i in indices {
        out.extend_from_slice(&i.to_ne_bytes());
    }
    out
}
