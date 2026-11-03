use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    time::{Duration, Instant},
};

const MAX_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShaderWatchError {
    Paths,
    TooLarge,
    Utf8,
    Io(std::io::ErrorKind),
}
pub struct ShaderWatcher {
    paths: Vec<PathBuf>,
    interval: Duration,
    next: Instant,
    source: String,
    last: String,
    scratch: Vec<u8>,
    initialized: bool,
}
impl ShaderWatcher {
    pub fn new(paths: Vec<PathBuf>, interval: Duration) -> Result<Self, ShaderWatchError> {
        if paths.is_empty() || paths.len() > 8 {
            return Err(ShaderWatchError::Paths);
        }
        Ok(Self {
            paths,
            interval: interval.clamp(Duration::from_millis(50), Duration::from_secs(60)),
            next: Instant::now(),
            source: String::new(),
            last: String::new(),
            scratch: Vec::new(),
            initialized: false,
        })
    }
    pub fn poll(&mut self) -> Result<Option<&str>, ShaderWatchError> {
        self.poll_at(Instant::now())
    }
    pub fn poll_at(&mut self, now: Instant) -> Result<Option<&str>, ShaderWatchError> {
        if now < self.next {
            return Ok(None);
        }
        self.next = now + self.interval;
        self.source.clear();
        for path in &self.paths {
            self.scratch.clear();
            let remaining = MAX_BYTES.saturating_sub(self.source.len());
            File::open(path)
                .and_then(|file| {
                    file.take(remaining as u64 + 1)
                        .read_to_end(&mut self.scratch)
                })
                .map_err(|e| ShaderWatchError::Io(e.kind()))?;
            if self.scratch.len() >= remaining {
                return Err(ShaderWatchError::TooLarge);
            }
            let text = std::str::from_utf8(&self.scratch).map_err(|_| ShaderWatchError::Utf8)?;
            self.source.push_str(text);
            self.source.push('\n');
        }
        if self.initialized && self.last == self.source {
            return Ok(None);
        }
        self.last.clear();
        self.last.push_str(&self.source);
        self.initialized = true;
        Ok(Some(&self.source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_revision_and_failed_bundle_are_atomic() {
        let root =
            std::env::temp_dir().join(format!("tacita-shader-watch-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let a = root.join("a.wgsl");
        let b = root.join("b.wgsl");
        std::fs::write(&a, "first").unwrap();
        std::fs::write(&b, "second").unwrap();
        let mut watch =
            ShaderWatcher::new(vec![a.clone(), b.clone()], Duration::from_millis(50)).unwrap();
        let now = Instant::now();
        assert_eq!(watch.poll_at(now).unwrap(), Some("first\nsecond\n"));
        assert_eq!(
            watch.poll_at(now + Duration::from_millis(60)).unwrap(),
            None
        );
        std::fs::write(&a, "changed").unwrap();
        std::fs::write(&b, [255]).unwrap();
        assert_eq!(
            watch.poll_at(now + Duration::from_millis(120)),
            Err(ShaderWatchError::Utf8)
        );
        std::fs::write(&b, "valid").unwrap();
        assert_eq!(
            watch.poll_at(now + Duration::from_millis(180)).unwrap(),
            Some("changed\nvalid\n")
        );
        std::fs::write(&b, vec![b'x'; MAX_BYTES]).unwrap();
        assert_eq!(
            watch.poll_at(now + Duration::from_millis(240)),
            Err(ShaderWatchError::TooLarge)
        );
        std::fs::remove_file(a).unwrap();
        std::fs::remove_file(b).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
