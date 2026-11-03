use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
struct Slot {
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    pending: Arc<AtomicBool>,
}
pub struct GpuTimer {
    queries: wgpu::QuerySet,
    slots: Vec<Slot>,
    sender: mpsc::Sender<Option<f64>>,
    receiver: mpsc::Receiver<Option<f64>>,
    period: f64,
    pub latest: Option<f64>,
}
impl GpuTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("GUI timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 6,
        });
        let slots = (0..3)
            .map(|_| Slot {
                resolve: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("timestamp resolve"),
                    size: 256,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("timestamp readback"),
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                pending: Arc::new(AtomicBool::new(false)),
            })
            .collect();
        let (sender, receiver) = mpsc::channel();
        Self {
            queries,
            slots,
            sender,
            receiver,
            period: queue.get_timestamp_period() as f64,
            latest: None,
        }
    }
    pub fn poll(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        for value in self.receiver.try_iter().flatten() {
            self.latest = Some(value);
        }
    }
    pub fn begin(&mut self, device: &wgpu::Device) -> Option<(usize, wgpu::QuerySet)> {
        self.poll(device);
        let index = self
            .slots
            .iter()
            .position(|s| !s.pending.load(Ordering::Acquire))?;
        Some((index, self.queries.clone()))
    }
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder, index: usize) {
        let slot = &self.slots[index];
        encoder.resolve_query_set(
            &self.queries,
            index as u32 * 2..index as u32 * 2 + 2,
            &slot.resolve,
            0,
        );
        encoder.copy_buffer_to_buffer(&slot.resolve, 0, &slot.readback, 0, 16);
    }
    pub fn map(&self, index: usize) {
        let slot = &self.slots[index];
        slot.pending.store(true, Ordering::Release);
        let pending = slot.pending.clone();
        let buffer = slot.readback.clone();
        let sender = self.sender.clone();
        let period = self.period;
        slot.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let mut value = None;
                if result.is_ok() {
                    if let Ok(data) = buffer.slice(..).get_mapped_range() {
                        let start = u64::from_le_bytes(core::array::from_fn(|i| data[i]));
                        let end = u64::from_le_bytes(core::array::from_fn(|i| data[i + 8]));
                        value = (end >= start).then(|| (end - start) as f64 * period / 1_000_000.);
                        drop(data);
                    }
                    buffer.unmap();
                }
                let _ = sender.send(value);
                pending.store(false, Ordering::Release);
            });
    }
}
