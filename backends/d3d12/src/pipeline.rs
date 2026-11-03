use windows::Win32::Graphics::Direct3D12::{
    D3D12_CLEAR_FLAG_STENCIL, D3D12_COMPARISON_FUNC_EQUAL, D3D12_DEPTH_STENCILOP_DESC,
    D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_COPY_SOURCE,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_RENDER_TARGET,
    D3D12_STENCIL_OP_DECR_SAT, D3D12_STENCIL_OP_INCR_SAT, D3D12_STENCIL_OP_KEEP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_D24_UNORM_S8_UINT;

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
use restricted_gui::rendering::{
    AlphaMode, CommandPlan, DrawCalls, DrawOptimization, GeometryRange,
};
#[path = "rendering_options.rs"]
mod rendering_options;
use crate::texture::{TextureSet, UploadContext};
use crate::{expect_resource, BackendError, Device, FrameSync, Queue, FRAMES_IN_FLIGHT};
use rendering_options::{PsoConfig, RasterSet};

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
}

pub(crate) struct Retired {
    pub(crate) fence: u64,
    pub(crate) kind: RetiredKind,
}

#[allow(dead_code)]
pub(crate) enum RetiredKind {
    Resource(ID3D12Resource),
    PipelineState(ID3D12PipelineState),
    CommandRecording(ID3D12CommandAllocator, ID3D12GraphicsCommandList),
    TextureSlot(ID3D12Resource, u32),
}

struct FrameSlot {
    allocator: ID3D12CommandAllocator,
    list: ID3D12GraphicsCommandList,
    ring: UploadRing,
    last_signaled: Option<u64>,
    stencil: Option<crate::compositor::Target>,
    timer: crate::timing::GpuTimer,
}

impl FrameSlot {
    fn new(device: &Device, queue: &Queue) -> Result<Self, BackendError> {
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
            stencil: None,
            timer: crate::timing::GpuTimer::new(device, queue)?,
        })
    }
}

pub(crate) struct RenderOptions<'a> {
    pub sampled: Option<&'a ID3D12Resource>,
    pub clear: Option<[f32; 4]>,
    pub color_mode: crate::ColorMode,
    pub scale_factor: f32,
    pub sample_count: u32,
    pub resolve: Option<(
        &'a ID3D12Resource,
        windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT,
    )>,
}

pub(crate) struct Pipeline {
    optimization: DrawOptimization,
    plan: CommandPlan,
    alpha_mode: AlphaMode,
    texture_alpha: Vec<(TextureId, AlphaMode)>,
    sources: (String, String),
    rasters: Vec<RasterSet>,
    device: Device,
    queue: Queue,
    root_signature: ID3D12RootSignature,
    pso: ID3D12PipelineState,
    variants: [ID3D12PipelineState; 5],
    managed: [ID3D12PipelineState; 6],
    textures: TextureSet,
    slots: Vec<FrameSlot>,
    current_slot: usize,
    retired: std::collections::VecDeque<Retired>,
    full_idle_waits: u64,
    free_rings: Vec<UploadRing>,
    minimum_ring: u64,
    shrink_rings: bool,
    stats: crate::RenderStats,
}
impl Default for RenderOptions<'_> {
    fn default() -> Self {
        Self {
            sampled: None,
            clear: None,
            color_mode: crate::ColorMode::Legacy,
            scale_factor: 1.,
            sample_count: 1,
            resolve: None,
        }
    }
}

impl Pipeline {
    pub(crate) fn new(
        device: &Device,
        queue: &Queue,
        sync: &mut FrameSync,
    ) -> Result<Self, BackendError> {
        let root_signature = create_root_signature(device)?;
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
        let managed_vs = shader::compile(
            &format!(
                "#define GUI_MANAGED_SRGB 1\n{}",
                shader::VERTEX_SHADER_SOURCE
            ),
            shader::VS_ENTRY,
            shader::VS_TARGET,
        )?;
        let managed_ps = shader::compile(
            &format!(
                "#define GUI_MANAGED_SRGB 1\n{}",
                shader::PIXEL_SHADER_SOURCE
            ),
            shader::PS_ENTRY,
            shader::PS_TARGET,
        )?;
        let managed = [
            create_pso(
                device,
                &root_signature,
                0,
                &managed_vs,
                &managed_ps,
                true,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                1,
                &managed_vs,
                &managed_ps,
                true,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                2,
                &managed_vs,
                &managed_ps,
                true,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                3,
                &managed_vs,
                &managed_ps,
                true,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                4,
                &managed_vs,
                &managed_ps,
                true,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                5,
                &managed_vs,
                &managed_ps,
                true,
                PsoConfig::default(),
            )?,
        ];
        let pso = create_pso(
            device,
            &root_signature,
            0,
            &vs_blob,
            &ps_blob,
            false,
            PsoConfig::default(),
        )?;
        let variants = [
            create_pso(
                device,
                &root_signature,
                1,
                &vs_blob,
                &ps_blob,
                false,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                2,
                &vs_blob,
                &ps_blob,
                false,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                3,
                &vs_blob,
                &ps_blob,
                false,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                4,
                &vs_blob,
                &ps_blob,
                false,
                PsoConfig::default(),
            )?,
            create_pso(
                device,
                &root_signature,
                5,
                &vs_blob,
                &ps_blob,
                false,
                PsoConfig::default(),
            )?,
        ];

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
            slots.push(FrameSlot::new(device, queue)?);
        }

        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            optimization: DrawOptimization::default(),
            plan: CommandPlan::default(),
            alpha_mode: AlphaMode::Straight,
            texture_alpha: Vec::new(),
            sources: (
                shader::VERTEX_SHADER_SOURCE.into(),
                shader::PIXEL_SHADER_SOURCE.into(),
            ),
            rasters: Vec::new(),
            root_signature,
            pso,
            variants,
            managed,
            textures,
            slots,
            current_slot: 0,
            retired,
            full_idle_waits: 0,
            free_rings: Vec::new(),
            minimum_ring: RING_INITIAL_CAPACITY,
            shrink_rings: false,
            stats: crate::RenderStats {
                allocations: FRAMES_IN_FLIGHT as u64,
                ..Default::default()
            },
        })
    }

    pub(crate) fn stats(&self) -> crate::RenderStats {
        let mut stats = self.stats;
        stats.upload_capacity = self.slots.iter().map(|s| s.ring.capacity).sum::<u64>();
        stats.pool_bytes = self.free_rings.iter().map(|s| s.capacity).sum();
        stats.pool_entries = self.free_rings.len() as u64;
        stats
    }
    pub(crate) fn reserve_upload_capacity(&mut self, bytes: u64) -> Result<(), BackendError> {
        if !(1024..=128 * 1024 * 1024).contains(&bytes) {
            return Err(BackendError::InvalidTextureData(
                "invalid upload reserve".into(),
            ));
        }
        self.minimum_ring = self.minimum_ring.max(bytes.next_power_of_two());
        Ok(())
    }
    pub(crate) fn set_upload_capacity(&mut self, bytes: u64) -> Result<(), BackendError> {
        if !(1024..=128 * 1024 * 1024).contains(&bytes) {
            return Err(BackendError::InvalidTextureData(
                "invalid upload capacity".into(),
            ));
        }
        self.minimum_ring = bytes.next_power_of_two();
        self.shrink_rings = true;
        Ok(())
    }
    pub(crate) fn trim_upload_pool(&mut self) {
        self.free_rings.clear();
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
        self.add_host_texture_encoded(
            sync,
            id,
            rgba,
            width,
            height,
            crate::TextureEncoding::Linear,
        )
    }
    pub(crate) fn add_host_texture_encoded(
        &mut self,
        sync: &mut FrameSync,
        id: TextureId,
        rgba: &[u8],
        width: u32,
        height: u32,
        encoding: crate::TextureEncoding,
    ) -> Result<(), BackendError> {
        self.sweep_retirements(sync);
        let mut ctx = UploadContext {
            device: &self.device,
            queue: &self.queue,
            sync,
            retired: &mut self.retired,
        };
        self.textures
            .add_host_texture_encoded(&mut ctx, id, rgba, width, height, encoding)?;
        self.texture_alpha.retain(|(key, _)| *key != id);
        Ok(())
    }

    pub(crate) fn aliases(&self, id: TextureId, target: &ID3D12Resource) -> bool {
        self.textures.resource_for(id) == Some(target)
    }
    pub(crate) fn texture_size(&self, id: TextureId) -> Option<[u32; 2]> {
        self.textures.size_for(id)
    }
    pub(crate) fn add_gpu_texture(
        &mut self,
        sync: &mut FrameSync,
        id: TextureId,
        resource: &ID3D12Resource,
    ) -> Result<(), BackendError> {
        self.sweep_retirements(sync);
        let mut ctx = UploadContext {
            device: &self.device,
            queue: &self.queue,
            sync,
            retired: &mut self.retired,
        };
        self.textures.add_gpu_texture(&mut ctx, id, resource)?;
        self.texture_alpha.retain(|(key, _)| *key != id);
        Ok(())
    }
    pub(crate) fn capture(
        &mut self,
        sync: &mut FrameSync,
        source: &ID3D12Resource,
        destination: &ID3D12Resource,
    ) -> Result<(), BackendError> {
        if source == destination {
            return Err(BackendError::InvalidTextureData(
                "capture source aliases its output".into(),
            ));
        }
        self.sweep_retirements(sync);
        let allocator: ID3D12CommandAllocator = unsafe {
            self.device
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
        }
        .map_err(BackendError::CommandAllocatorCreation)?;
        let list: ID3D12GraphicsCommandList = unsafe {
            self.device
                .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
        }
        .map_err(BackendError::CommandListCreation)?;
        crate::record_transition_barrier(
            &list,
            source,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
        );
        crate::record_transition_barrier(
            &list,
            destination,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            D3D12_RESOURCE_STATE_COPY_DEST,
        );
        unsafe {
            list.CopyResource(destination, source);
        }
        crate::record_transition_barrier(
            &list,
            source,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
        crate::record_transition_barrier(
            &list,
            destination,
            D3D12_RESOURCE_STATE_COPY_DEST,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        );
        unsafe { list.Close() }.map_err(|e| BackendError::CommandRecording("capture close", e))?;
        let submitted: ID3D12CommandList = list.clone().into();
        unsafe {
            self.queue.ExecuteCommandLists(&[Some(submitted)]);
        }
        let fence = sync.signal(&self.queue)?;
        self.retired.push_back(Retired {
            fence,
            kind: RetiredKind::CommandRecording(allocator, list),
        });
        self.retired.push_back(Retired {
            fence,
            kind: RetiredKind::Resource(source.clone()),
        });
        Ok(())
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
        options: RenderOptions<'_>,
    ) -> Result<(), BackendError> {
        self.sweep_retirements(sync);
        let RenderOptions {
            sampled: sampled_target,
            clear,
            color_mode,
            scale_factor,
            sample_count,
            resolve,
        } = options;

        let [width, height] = viewport_size;
        if width <= 0.0
            || height <= 0.0
            || (clear.is_none() && (draw_data.vertices.is_empty() || draw_data.indices.is_empty()))
        {
            return Ok(());
        }

        let raster = if self.optimization.enabled()
            || sample_count != 1
            || self.alpha_mode != AlphaMode::Straight
        {
            Some(self.ensure_raster(sample_count)?)
        } else {
            None
        };
        if self.optimization.enabled() {
            self.plan
                .prepare(draw_data, self.optimization, scale_factor)
                .map_err(|e| BackendError::InvalidTextureData(format!("draw plan: {e:?}")))?;
        }
        let triangles = !self.optimization.enabled()
            || self
                .plan
                .draws()
                .iter()
                .any(|d| matches!(d.geometry, GeometryRange::Triangles { .. }));
        let alpha_banks = if self.alpha_mode != AlphaMode::Straight
            || draw_data.commands.iter().any(|cmd| {
                rendering_options::texture_alpha(&self.texture_alpha, cmd.texture)
                    != AlphaMode::Straight
            })
            || draw_data.effects.iter().any(|e| {
                rendering_options::texture_alpha(
                    &self.texture_alpha,
                    e.mask_texture(3).unwrap_or(TextureId::WHITE),
                ) != AlphaMode::Straight
            }) {
            8
        } else {
            1
        };
        let slot_index = self.current_slot;
        if let Some(value) = self.slots[slot_index].last_signaled {
            self.wait_tracked(sync, value)?;
        }
        let slot = &mut self.slots[slot_index];
        let stencil_active = draw_data.effects.iter().any(|e| e.stencil != [0; 2]);
        if stencil_active
            && slot.stencil.as_ref().is_none_or(|t| {
                t.size != [width.ceil() as u32, height.ceil() as u32] || t.samples != sample_count
            })
        {
            slot.stencil = Some(crate::compositor::Target::new_samples(
                &self.device,
                [width.ceil() as u32, height.ceil() as u32],
                DXGI_FORMAT_D24_UNORM_S8_UINT,
                true,
                sample_count,
            )?);
        }
        let dsv = if stencil_active {
            slot.stencil.as_ref().map(|t| t.handle)
        } else {
            None
        };

        let vertex_bytes = vertex_bytes(if triangles { draw_data.vertices } else { &[] });
        let index_bytes = index_bytes(if triangles { draw_data.indices } else { &[] });
        let quad_bytes = rendering_options::instance_bytes(if self.optimization.enabled() {
            self.plan.instances()
        } else {
            &[]
        });
        let cb_offset = 0u64;
        let vb_offset = RING_ALIGNMENT;
        let ib_offset = align_up(vb_offset + vertex_bytes.len() as u64, RING_ALIGNMENT);
        let quad_offset = align_up(ib_offset + index_bytes.len() as u64, RING_ALIGNMENT);
        let effects_offset = align_up(quad_offset + quad_bytes.len() as u64, RING_ALIGNMENT);
        let effect_stride = restricted_gui::effects::EFFECT_UNIFORM_SIZE as u64;
        let required =
            effects_offset + (draw_data.effects.len() as u64 + 1) * effect_stride * alpha_banks;
        let desired = required.max(self.minimum_ring).next_power_of_two();
        if desired > 128 * 1024 * 1024 {
            return Err(BackendError::InvalidTextureData(
                "upload ring exceeds 128 MiB".into(),
            ));
        }
        if slot.ring.capacity < desired || (self.shrink_rings && slot.ring.capacity > desired) {
            let found = self
                .free_rings
                .iter()
                .enumerate()
                .filter(|(_, ring)| {
                    ring.capacity >= desired && (!self.shrink_rings || ring.capacity == desired)
                })
                .min_by_key(|(_, ring)| ring.capacity)
                .map(|(i, _)| i);
            let replacement = if let Some(index) = found {
                self.stats.reuses += 1;
                self.free_rings.swap_remove(index)
            } else {
                self.stats.allocations += 1;
                UploadRing::new(&self.device, desired)?
            };
            let old = std::mem::replace(&mut slot.ring, replacement);
            if self.free_rings.len() < 4
                && self.free_rings.iter().map(|r| r.capacity).sum::<u64>() + old.capacity
                    <= 128 * 1024 * 1024
            {
                self.free_rings.push(old);
            }
        }
        if slot.last_signaled.is_some() {
            self.stats.gpu_ms = slot.timer.read()?;
        }
        self.stats.uploaded_bytes = required;
        self.stats.draw_calls = 0;

        let projection_columns: [f32; 16] = [
            2.0 * scale_factor / width,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 * scale_factor / height,
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
            std::ptr::copy_nonoverlapping(
                quad_bytes.as_ptr(),
                base.add(quad_offset as usize),
                quad_bytes.len(),
            );
            for flags in 0..alpha_banks {
                for (i, effect) in core::iter::once(restricted_gui::effects::DrawEffects::default())
                    .chain(draw_data.effects.iter().copied())
                    .enumerate()
                {
                    let bytes = effect.uniform_bytes_with_alpha(flags as u32);
                    let offset = effects_offset
                        + (flags * (draw_data.effects.len() as u64 + 1) + i as u64) * effect_stride;
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        base.add(offset as usize),
                        bytes.len(),
                    );
                }
            }
        }

        unsafe {
            slot.allocator
                .Reset()
                .map_err(|e| BackendError::CommandRecording("command allocator reset", e))?;
            slot.list
                .Reset(&slot.allocator, None)
                .map_err(|e| BackendError::CommandRecording("command list reset", e))?;

            slot.timer.begin(&slot.list);
            if let Some(target) = sampled_target {
                crate::record_transition_barrier(
                    &slot.list,
                    target,
                    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                    D3D12_RESOURCE_STATE_RENDER_TARGET,
                );
            }
            if let Some(color) = clear {
                slot.list.ClearRenderTargetView(rtv, &color, None);
            }
            slot.list
                .SetPipelineState(if color_mode == crate::ColorMode::Srgb {
                    &self.managed[0]
                } else {
                    &self.pso
                });
            slot.list.SetGraphicsRootSignature(&self.root_signature);
            slot.list
                .SetDescriptorHeaps(&[Some(self.textures.heap().clone())]);
            slot.list
                .OMSetRenderTargets(1, Some(&rtv), false, dsv.as_ref().map(|v| v as *const _));
            if let Some(view) = dsv {
                slot.list
                    .ClearDepthStencilView(view, D3D12_CLEAR_FLAG_STENCIL, 1., 0, None);
            }
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

            let mut bound_mode = None;
            'commands: for (cmd, geometry) in
                DrawCalls::new(draw_data, self.optimization.enabled().then_some(&self.plan))
            {
                if cmd.elem_count == 0 {
                    continue;
                }
                let Some(scissor) =
                    scissor_rect(cmd.clip_rect.map(|v| v * scale_factor), width, height)
                else {
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
                let effect = if cmd.effects == 0 {
                    restricted_gui::effects::DrawEffects::default()
                } else if let Some(effect) = draw_data.effects.get(cmd.effects as usize - 1) {
                    *effect
                } else {
                    eprintln!("[d3d12] skipping invalid effect index {}", cmd.effects);
                    continue;
                };
                if effect.stencil[0] > 8 || effect.stencil[1] > 2 {
                    continue;
                }
                let mode = if stencil_active {
                    match effect.stencil[1] {
                        1 => 3,
                        2 => 4,
                        _ => {
                            if effect.filter.overwrite != 0 {
                                5
                            } else {
                                2
                            }
                        }
                    }
                } else if effect.filter.overwrite != 0 {
                    1
                } else {
                    0
                };
                let instanced = matches!(geometry, GeometryRange::Quads { .. });
                let key = [mode as u32, effect.stencil[0], u32::from(instanced)];
                if bound_mode != Some(key) {
                    slot.list.SetPipelineState(if let Some(index) = raster {
                        self.rasters[index].pipeline(color_mode, instanced, mode)
                    } else if color_mode == crate::ColorMode::Srgb {
                        &self.managed[mode]
                    } else if mode == 0 {
                        &self.pso
                    } else {
                        &self.variants[mode - 1]
                    });
                    slot.list.OMSetStencilRef(effect.stencil[0]);
                    bound_mode = Some(key);
                }
                let mut masks = [srv; 4];
                for (i, handle) in masks.iter_mut().enumerate() {
                    let id = effect.mask_texture(i).unwrap_or(TextureId::WHITE);
                    let Some(mask_srv) = self.textures.srv_for(id) else {
                        eprintln!("[d3d12] skipping unregistered mask texture {id:?}");
                        continue 'commands;
                    };
                    *handle = mask_srv;
                }
                slot.list.RSSetScissorRects(&[scissor]);
                slot.list.SetGraphicsRootDescriptorTable(1, srv);
                slot.list.SetGraphicsRootConstantBufferView(
                    2,
                    slot.ring.gpu_base
                        + effects_offset
                        + (restricted_gui::rendering::alpha_flags(
                            rendering_options::texture_alpha(&self.texture_alpha, cmd.texture),
                            self.alpha_mode,
                            rendering_options::texture_alpha(
                                &self.texture_alpha,
                                effect.mask_texture(3).unwrap_or(TextureId::WHITE),
                            ),
                        ) as u64
                            * (draw_data.effects.len() as u64 + 1)
                            + cmd.effects as u64)
                            * effect_stride,
                );
                for (i, handle) in masks.into_iter().enumerate() {
                    slot.list
                        .SetGraphicsRootDescriptorTable(3 + i as u32, handle);
                }
                match geometry {
                    GeometryRange::Triangles {
                        first_index,
                        index_count,
                    } => {
                        slot.list.IASetVertexBuffers(0, Some(&[vertex_view]));
                        slot.list
                            .DrawIndexedInstanced(index_count, 1, first_index, 0, 0);
                    }
                    GeometryRange::Quads {
                        first_instance,
                        instance_count,
                    } => {
                        let view = D3D12_VERTEX_BUFFER_VIEW {
                            BufferLocation: slot.ring.gpu_base + quad_offset,
                            SizeInBytes: quad_bytes.len() as u32,
                            StrideInBytes: 80,
                        };
                        slot.list.IASetVertexBuffers(0, Some(&[view]));
                        slot.list
                            .DrawInstanced(6, instance_count, 0, first_instance);
                    }
                }
                self.stats.draw_calls += 1;
            }

            if let Some(source) = sampled_target {
                if let Some((target, format)) = resolve {
                    use windows::Win32::Graphics::Direct3D12::{
                        D3D12_RESOURCE_STATE_RESOLVE_DEST, D3D12_RESOURCE_STATE_RESOLVE_SOURCE,
                    };
                    crate::record_transition_barrier(
                        &slot.list,
                        source,
                        D3D12_RESOURCE_STATE_RENDER_TARGET,
                        D3D12_RESOURCE_STATE_RESOLVE_SOURCE,
                    );
                    crate::record_transition_barrier(
                        &slot.list,
                        target,
                        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                        D3D12_RESOURCE_STATE_RESOLVE_DEST,
                    );
                    slot.list.ResolveSubresource(target, 0, source, 0, format);
                    crate::record_transition_barrier(
                        &slot.list,
                        source,
                        D3D12_RESOURCE_STATE_RESOLVE_SOURCE,
                        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                    );
                    crate::record_transition_barrier(
                        &slot.list,
                        target,
                        D3D12_RESOURCE_STATE_RESOLVE_DEST,
                        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                    );
                } else {
                    crate::record_transition_barrier(
                        &slot.list,
                        source,
                        D3D12_RESOURCE_STATE_RENDER_TARGET,
                        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                    );
                }
            }
            slot.timer.end(&slot.list);
            slot.list
                .Close()
                .map_err(|e| BackendError::CommandRecording("command list close", e))?;
            let list: ID3D12CommandList = slot.list.clone().into();
            self.queue.ExecuteCommandLists(&[Some(list)]);
        }
        let fence = sync.signal(&self.queue)?;
        slot.last_signaled = Some(fence);
        if let (Some(source), Some((target, _))) = (sampled_target, resolve) {
            for resource in [source, target] {
                self.retired.push_back(Retired {
                    fence,
                    kind: RetiredKind::Resource(resource.clone()),
                });
            }
        }
        self.current_slot = (slot_index + 1) % self.slots.len();
        Ok(())
    }
}

fn create_root_signature(device: &Device) -> Result<ID3D12RootSignature, BackendError> {
    let ranges: [D3D12_DESCRIPTOR_RANGE; 5] = core::array::from_fn(|i| D3D12_DESCRIPTOR_RANGE {
        RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
        NumDescriptors: 1,
        BaseShaderRegister: i as u32,
        RegisterSpace: 0,
        OffsetInDescriptorsFromTableStart: 0,
    });
    let parameters: [D3D12_ROOT_PARAMETER; 7] = core::array::from_fn(|i| {
        if i == 0 || i == 2 {
            D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR {
                        ShaderRegister: if i == 0 { 0 } else { 1 },
                        RegisterSpace: 0,
                    },
                },
                ShaderVisibility: if i == 0 {
                    D3D12_SHADER_VISIBILITY_VERTEX
                } else {
                    D3D12_SHADER_VISIBILITY_PIXEL
                },
            }
        } else {
            let range = if i == 1 { 0 } else { i - 2 };
            D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                        NumDescriptorRanges: 1,
                        pDescriptorRanges: &ranges[range],
                    },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
            }
        }
    });
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
    mode: usize,
    vs_blob: &windows::Win32::Graphics::Direct3D::ID3DBlob,
    ps_blob: &windows::Win32::Graphics::Direct3D::ID3DBlob,
    managed: bool,
    configuration: PsoConfig,
) -> Result<ID3D12PipelineState, BackendError> {
    let bytecode = |blob: &windows::Win32::Graphics::Direct3D::ID3DBlob| D3D12_SHADER_BYTECODE {
        pShaderBytecode: unsafe { blob.GetBufferPointer() },
        BytecodeLength: unsafe { blob.GetBufferSize() },
    };

    let mut blend_desc = D3D12_BLEND_DESC::default();
    blend_desc.RenderTarget[0] = D3D12_RENDER_TARGET_BLEND_DESC {
        BlendEnable: (mode != 1 && mode != 3 && mode != 4 && mode != 5).into(),
        SrcBlend: if configuration.alpha == AlphaMode::Premultiplied {
            D3D12_BLEND_ONE
        } else {
            D3D12_BLEND_SRC_ALPHA
        },
        DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
        BlendOp: D3D12_BLEND_OP_ADD,
        SrcBlendAlpha: D3D12_BLEND_ONE,
        DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
        BlendOpAlpha: D3D12_BLEND_OP_ADD,
        RenderTargetWriteMask: if mode == 3 || mode == 4 {
            0
        } else {
            D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8
        },
        ..Default::default()
    };

    let rasterizer_desc = D3D12_RASTERIZER_DESC {
        FillMode: D3D12_FILL_MODE_SOLID,
        CullMode: D3D12_CULL_MODE_NONE,
        DepthClipEnable: true.into(),
        MultisampleEnable: (configuration.samples > 1).into(),
        ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
        ..Default::default()
    };

    let face = D3D12_DEPTH_STENCILOP_DESC {
        StencilFailOp: D3D12_STENCIL_OP_KEEP,
        StencilDepthFailOp: D3D12_STENCIL_OP_KEEP,
        StencilPassOp: match mode {
            3 => D3D12_STENCIL_OP_INCR_SAT,
            4 => D3D12_STENCIL_OP_DECR_SAT,
            _ => D3D12_STENCIL_OP_KEEP,
        },
        StencilFunc: D3D12_COMPARISON_FUNC_EQUAL,
    };
    let depth_stencil_desc = D3D12_DEPTH_STENCIL_DESC {
        DepthEnable: false.into(),
        DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
        DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        StencilEnable: (mode >= 2).into(),
        StencilReadMask: 255,
        StencilWriteMask: if mode == 3 || mode == 4 { 255 } else { 0 },
        FrontFace: face,
        BackFace: face,
    };

    let mut rtv_formats = [DXGI_FORMAT_UNKNOWN; 8];
    rtv_formats[0] = if managed {
        windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
    } else {
        DXGI_FORMAT_R8G8B8A8_UNORM
    };

    let quad_elements: [D3D12_INPUT_ELEMENT_DESC; 12] = core::array::from_fn(|i| {
        let mut item = INPUT_LAYOUT[i % 3];
        item.SemanticIndex = (i / 3) as u32;
        item.AlignedByteOffset += (i / 3) as u32 * 20;
        item.InputSlotClass =
            windows::Win32::Graphics::Direct3D12::D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA;
        item.InstanceDataStepRate = 1;
        item
    });
    let elements = if configuration.instanced {
        quad_elements.as_slice()
    } else {
        INPUT_LAYOUT.as_slice()
    };
    let pso_desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
        pRootSignature: std::mem::ManuallyDrop::new(Some(root_signature.clone())),
        VS: bytecode(vs_blob),
        PS: bytecode(ps_blob),
        BlendState: blend_desc,
        SampleMask: u32::MAX,
        RasterizerState: rasterizer_desc,
        DepthStencilState: depth_stencil_desc,
        InputLayout: D3D12_INPUT_LAYOUT_DESC {
            pInputElementDescs: elements.as_ptr(),
            NumElements: elements.len() as u32,
        },
        PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        NumRenderTargets: 1,
        RTVFormats: rtv_formats,
        DSVFormat: if mode >= 2 {
            DXGI_FORMAT_D24_UNORM_S8_UINT
        } else {
            DXGI_FORMAT_UNKNOWN
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: configuration.samples,
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

fn vertex_bytes(vertices: &[DrawVert]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr().cast::<u8>(),
            std::mem::size_of_val(vertices),
        )
    }
}

fn index_bytes(indices: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            indices.as_ptr().cast::<u8>(),
            std::mem::size_of_val(indices),
        )
    }
}
