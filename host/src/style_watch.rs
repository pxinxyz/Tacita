use tacita::{
    context::Context,
    persistence::{decode_theme, StateError},
    theme::ThemeTokens,
};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReloadStatus {
    Waiting,
    Unchanged,
    Reloaded,
    Invalid(StateError),
    Io(std::io::ErrorKind),
}
pub struct StyleWatcher {
    path: PathBuf,
    interval: Duration,
    next: Instant,
    bytes: Vec<u8>,
    last: Vec<u8>,
    last_validation: ReloadStatus,
    status: ReloadStatus,
}
impl StyleWatcher {
    pub fn new(path: impl Into<PathBuf>, interval: Duration) -> Self {
        Self {
            path: path.into(),
            interval: interval.clamp(Duration::from_millis(50), Duration::from_secs(60)),
            next: Instant::now(),
            bytes: Vec::with_capacity(16385),
            last: Vec::with_capacity(16385),
            last_validation: ReloadStatus::Waiting,
            status: ReloadStatus::Waiting,
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn status(&self) -> &ReloadStatus {
        &self.status
    }
    pub fn poll(&mut self) -> Option<ThemeTokens> {
        self.poll_at(Instant::now())
    }
    pub fn poll_at(&mut self, now: Instant) -> Option<ThemeTokens> {
        if now < self.next {
            return None;
        }
        self.next = now + self.interval;
        self.bytes.clear();
        let result =
            File::open(&self.path).and_then(|file| file.take(16385).read_to_end(&mut self.bytes));
        if let Err(error) = result {
            self.status = ReloadStatus::Io(error.kind());
            return None;
        }
        if self.bytes == self.last && self.last_validation != ReloadStatus::Waiting {
            self.status = match &self.last_validation {
                ReloadStatus::Reloaded => ReloadStatus::Unchanged,
                status => status.clone(),
            };
            return None;
        }
        self.last.clear();
        self.last.extend_from_slice(&self.bytes);
        let theme = match decode_theme(&self.bytes) {
            Ok(tokens) => {
                self.status = ReloadStatus::Reloaded;
                Some(tokens)
            }
            Err(error) => {
                self.status = ReloadStatus::Invalid(error);
                None
            }
        };
        self.last_validation = self.status.clone();
        theme
    }
    pub fn poll_apply(&mut self, context: &mut Context) -> bool {
        if let Some(tokens) = self.poll() {
            context.set_theme_tokens(tokens);
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tacita::persistence::encode_theme;
    #[test]
    fn edits_replacements_and_errors_keep_validated_state() {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "tacita-style-{}-{}.theme",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut theme = ThemeTokens::default();
        std::fs::write(&path, encode_theme(theme).unwrap()).unwrap();
        let now = Instant::now();
        let mut watch = StyleWatcher::new(&path, Duration::from_millis(100));
        assert_eq!(watch.poll_at(now + Duration::from_millis(1)), Some(theme));
        assert_eq!(watch.poll_at(now + Duration::from_millis(200)), None);
        assert_eq!(watch.status(), &ReloadStatus::Unchanged);
        std::fs::write(&path, "invalid").unwrap();
        assert!(watch.poll_at(now + Duration::from_millis(400)).is_none());
        assert!(matches!(watch.status(), ReloadStatus::Invalid(_)));
        theme.corner_rounding = 2.5;
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, encode_theme(theme).unwrap()).unwrap();
        assert_eq!(watch.poll_at(now + Duration::from_millis(600)), Some(theme));
        std::fs::remove_file(&path).unwrap();
        assert!(watch.poll_at(now + Duration::from_millis(800)).is_none());
        assert!(matches!(watch.status(), ReloadStatus::Io(_)));
        std::fs::write(&path, encode_theme(theme).unwrap()).unwrap();
        assert_eq!(watch.poll_at(now + Duration::from_millis(1000)), None);
        std::fs::write(&path, vec![b'x'; 16385]).unwrap();
        assert!(watch.poll_at(now + Duration::from_millis(1200)).is_none());
        assert_eq!(watch.status(), &ReloadStatus::Invalid(StateError::Limit));
        std::fs::remove_file(&path).unwrap();
    }
}
