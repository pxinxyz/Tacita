use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Stage {
    Idle,
    Settings,
    Widgets,
    Atlas,
    Upload,
    Acquire,
    Record,
    Submit,
    Present,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Settings => "settings",
            Self::Widgets => "widget declaration",
            Self::Atlas => "atlas sync",
            Self::Upload => "geometry/effect upload",
            Self::Acquire => "surface acquire",
            Self::Record => "draw recording",
            Self::Submit => "queue submit",
            Self::Present => "present",
        }
    }
    fn decode(value: u64) -> Self {
        match value & 255 {
            1 => Self::Settings,
            2 => Self::Widgets,
            3 => Self::Atlas,
            4 => Self::Upload,
            5 => Self::Acquire,
            6 => Self::Record,
            7 => Self::Submit,
            8 => Self::Present,
            _ => Self::Idle,
        }
    }
}
struct Shared {
    origin: Instant,
    stamp: AtomicU64,
    frame: AtomicU64,
}
#[derive(Clone, Default)]
pub struct Progress(Option<Arc<Shared>>);
impl Progress {
    pub fn mark(&self, stage: Stage) {
        if let Some(shared) = &self.0 {
            let elapsed = shared.origin.elapsed().as_millis() as u64;
            shared
                .stamp
                .store((elapsed << 8) | stage as u64, Ordering::Release);
        }
    }
    pub fn frame(&self, frame: u64) {
        if let Some(shared) = &self.0 {
            shared.frame.store(frame, Ordering::Relaxed);
        }
    }
}
pub struct FrameWatchdog {
    pub progress: Progress,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl FrameWatchdog {
    pub fn new(enabled: bool) -> Self {
        if !enabled {
            return Self {
                progress: Progress::default(),
                stop: None,
                thread: None,
            };
        }
        let shared = Arc::new(Shared {
            origin: Instant::now(),
            stamp: AtomicU64::new(0),
            frame: AtomicU64::new(0),
        });
        let progress = Progress(Some(shared.clone()));
        let (stop, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut reported = None;
            while matches!(
                receiver.recv_timeout(Duration::from_secs(1)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                let stamp = shared.stamp.load(Ordering::Acquire);
                let stage = Stage::decode(stamp);
                let age = (shared.origin.elapsed().as_millis() as u64).saturating_sub(stamp >> 8);
                if age >= 5_000 {
                    let period = age / 5_000;
                    if reported != Some((stamp, period)) {
                        eprintln!(
                            "[watchdog] frame {}: no progress in {} for {:.1}s",
                            shared.frame.load(Ordering::Relaxed),
                            stage.name(),
                            age as f64 / 1000.
                        );
                        reported = Some((stamp, period));
                    }
                }
            }
        });
        Self {
            progress,
            stop: Some(stop),
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_monitor_records_stage_and_stops_on_drop() {
        let watchdog = FrameWatchdog::new(true);
        watchdog.progress.frame(12);
        watchdog.progress.mark(Stage::Submit);
        let shared = watchdog.progress.0.as_ref().unwrap();
        assert!(matches!(
            Stage::decode(shared.stamp.load(Ordering::Acquire)),
            Stage::Submit
        ));
        assert_eq!(shared.frame.load(Ordering::Relaxed), 12);
        drop(watchdog);
    }

    #[test]
    fn disabled_monitor_creates_no_thread_or_shared_storage() {
        let watchdog = FrameWatchdog::new(false);
        watchdog.progress.mark(Stage::Acquire);
        assert!(watchdog.progress.0.is_none());
        assert!(watchdog.thread.is_none());
    }
}
impl Drop for FrameWatchdog {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
