use super::{seconds, Easing};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub easing: Easing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackMode {
    Once,
    Loop,
    PingPong,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineOptions {
    pub mode: PlaybackMode,
    pub speed: f32,
    pub playing: bool,
    pub revision: u64,
}
impl Default for TimelineOptions {
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Once,
            speed: 1.0,
            playing: true,
            revision: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Timeline {
    time: f64,
    revision: Option<u64>,
}
impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}
impl Timeline {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            revision: None,
        }
    }
    pub fn seek(&mut self, time: f32) {
        assert!(time.is_finite() && time >= 0.0, "invalid timeline position");
        self.time = f64::from(time);
    }
    pub fn advance(&mut self, dt: f32, duration: f32, options: TimelineOptions) -> f32 {
        assert!(
            duration.is_finite() && duration >= 0.0 && options.speed.is_finite(),
            "invalid timeline controls"
        );
        if self.revision != Some(options.revision) {
            if self.revision.is_some() || options.speed < 0.0 {
                self.time = if options.speed < 0.0 {
                    f64::from(duration)
                } else {
                    0.0
                };
            }
            self.revision = Some(options.revision);
        }
        if options.playing {
            self.time += f64::from(seconds(dt)) * f64::from(options.speed);
        }
        let duration = f64::from(duration);
        if duration == 0.0 {
            self.time = 0.0;
            return 0.0;
        }
        match options.mode {
            PlaybackMode::Once => {
                self.time = self.time.clamp(0.0, duration);
                self.time as f32
            }
            PlaybackMode::Loop | PlaybackMode::PingPong => {
                let period = if options.mode == PlaybackMode::Loop {
                    duration
                } else {
                    2.0 * duration
                };
                self.time %= period;
                if self.time < 0.0 {
                    self.time += period;
                }
                if self.time <= duration {
                    self.time as f32
                } else {
                    (period - self.time) as f32
                }
            }
        }
    }
}

pub fn valid_keyframes_with(count: usize, get: impl Fn(usize) -> Keyframe) -> bool {
    if count == 0 {
        return false;
    }
    let mut previous = -1.0;
    for i in 0..count {
        let key = get(i);
        if !key.time.is_finite()
            || !key.value.is_finite()
            || key.time < 0.0
            || (i == 0 && key.time != 0.0)
            || key.time <= previous
        {
            return false;
        }
        previous = key.time;
    }
    true
}

pub fn sample_keyframes(keys: &[Keyframe], time: f32) -> f32 {
    sample_keyframes_with(keys.len(), |i| keys[i], time)
}

pub fn sample_keyframes_with(count: usize, get: impl Fn(usize) -> Keyframe, time: f32) -> f32 {
    assert!(
        time.is_finite() && valid_keyframes_with(count, &get),
        "invalid keyframe track"
    );
    sample_valid(count, get, time)
}
pub(super) fn sample_valid(count: usize, get: impl Fn(usize) -> Keyframe, time: f32) -> f32 {
    let first = get(0);
    if time <= 0.0 || count == 1 {
        return first.value;
    }
    let last = get(count - 1);
    if time >= last.time {
        return last.value;
    }
    let (mut lo, mut hi) = (0, count - 1);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if get(mid).time <= time {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (a, b) = (get(lo), get(hi));
    let t = f64::from(b.easing.sample((time - a.time) / (b.time - a.time)));
    (f64::from(a.value) * (1.0 - t) + f64::from(b.value) * t) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    const KEYS: [Keyframe; 3] = [
        Keyframe {
            time: 0.0,
            value: 0.0,
            easing: Easing::Linear,
        },
        Keyframe {
            time: 1.0,
            value: 2.0,
            easing: Easing::Linear,
        },
        Keyframe {
            time: 2.0,
            value: 0.0,
            easing: Easing::QuadIn,
        },
    ];
    #[test]
    fn samples_segments_and_playback_controls() {
        assert_eq!(sample_keyframes(&KEYS, 0.5), 1.0);
        assert_eq!(sample_keyframes(&KEYS, 1.5), 1.5);
        assert_eq!(sample_keyframes(&KEYS, 9.0), 0.0);
        let mut clock = Timeline::new();
        let options = TimelineOptions {
            mode: PlaybackMode::PingPong,
            ..TimelineOptions::default()
        };
        assert_eq!(clock.advance(2.5, 2.0, options), 1.5);
        assert_eq!(
            clock.advance(
                1.0,
                2.0,
                TimelineOptions {
                    playing: false,
                    ..options
                }
            ),
            1.5
        );
        assert_eq!(
            clock.advance(
                0.5,
                2.0,
                TimelineOptions {
                    revision: 1,
                    ..options
                }
            ),
            0.5
        );
        assert_eq!(
            clock.advance(
                0.25,
                2.0,
                TimelineOptions {
                    speed: -1.0,
                    ..TimelineOptions {
                        revision: 1,
                        ..options
                    }
                }
            ),
            0.25
        );
        clock.seek(1.0);
        assert_eq!(
            clock.advance(
                f32::NAN,
                2.0,
                TimelineOptions {
                    revision: 1,
                    ..options
                }
            ),
            1.0
        );
    }
    #[test]
    fn looping_reverse_playback_and_single_key_tracks() {
        let mut clock = Timeline::new();
        let options = TimelineOptions {
            mode: PlaybackMode::Loop,
            speed: -1.0,
            ..TimelineOptions::default()
        };
        assert_eq!(clock.advance(0.25, 1.0, options), 0.75);
        assert_eq!(clock.advance(1.0, 1.0, options), 0.75);
        assert_eq!(clock.advance(0.75, 1.0, options), 0.0);
        assert_eq!(sample_keyframes(&KEYS[..1], 100.0), 0.0);
        assert_eq!(clock.advance(f32::MAX, 0.0, options), 0.0);
    }
    #[test]
    fn invalid_tracks_are_rejected() {
        assert!(!valid_keyframes_with(0, |_| KEYS[0]));
        assert!(!valid_keyframes_with(2, |_| KEYS[0]));
        assert!(!valid_keyframes_with(1, |_| Keyframe {
            time: 1.0,
            ..KEYS[0]
        }));
        assert!(!valid_keyframes_with(1, |_| Keyframe {
            value: f32::NAN,
            ..KEYS[0]
        }));
    }
}
