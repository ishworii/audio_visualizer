use anyhow::Context;
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use rodio::{OutputStream, Sink, Source};
use std::collections::VecDeque;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use std::thread;

const OUT_SAMPLE_RATE: u32 = 44_100;

// ── Public struct ─────────────────────────────────────────────────────────────

pub struct UrlStream {
    consumer: HeapConsumer<f32>,   // visualization samples (filled by playback, not decode)
    window: VecDeque<f32>,
    pub sample_rate: u32,
    _reader: thread::JoinHandle<()>,
    _audio_stream: OutputStream,   // dropping this stops audio
}

impl UrlStream {
    pub fn start(url: &str, fft_size: usize) -> anyhow::Result<Self> {
        // audio ring buffer: ffmpeg decode → RingSource → rodio
        let (audio_prod, audio_cons) = HeapRb::<f32>::new(OUT_SAMPLE_RATE as usize * 2).split();

        // viz ring buffer: filled by RingSource *at playback time* so viz = what's playing
        let (viz_prod, viz_cons) = HeapRb::<f32>::new(fft_size * 8).split();

        let (_audio_stream, handle) =
            OutputStream::try_default().context("Failed to open audio output device")?;
        let sink = Sink::try_new(&handle).context("Failed to create audio sink")?;
        sink.append(RingSource::new(audio_cons, viz_prod));
        sink.detach();

        let url = url.replace('\\', "");
        let url = url.trim().to_string();

        let _reader = thread::spawn(move || {
            if let Err(e) = run_pipeline(&url, audio_prod) {
                eprintln!("[url] error: {e}");
            }
        });

        Ok(Self {
            consumer: viz_cons,
            window: VecDeque::from(vec![0.0f32; fft_size]),
            sample_rate: OUT_SAMPLE_RATE,
            _reader,
            _audio_stream,
        })
    }

    pub fn read_window(&mut self, out: &mut Vec<f32>, size: usize) {
        while let Some(s) = self.consumer.pop() {
            self.window.push_back(s);
            if self.window.len() > size {
                self.window.pop_front();
            }
        }
        out.clear();
        out.extend(self.window.iter().copied());
        while out.len() < size {
            out.push(0.0);
        }
    }
}

// ── Background pipeline ───────────────────────────────────────────────────────

/// Downloads the audio, then decodes with ffmpeg at realtime speed.
/// Samples go into the audio ring buffer only; the viz buffer is filled
/// by RingSource at the moment rodio actually plays each sample.
fn run_pipeline(url: &str, mut audio_prod: HeapProducer<f32>) -> anyhow::Result<()> {
    let downloaded = download(url)?;
    eprintln!("[url] starting playback + visualization…");

    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-re",                                    // realtime speed
            "-i", downloaded.to_str().unwrap(),
            "-vn",
            "-f",  "f32le",
            "-ac", "1",
            "-ar", &OUT_SAMPLE_RATE.to_string(),
            "-loglevel", "quiet",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn ffmpeg — install with: brew install ffmpeg")?;

    let stdout = ffmpeg.stdout.take().expect("ffmpeg stdout not piped");
    let mut reader = BufReader::new(stdout);
    let mut bytes = [0u8; 4];

    loop {
        match reader.read_exact(&mut bytes) {
            Ok(()) => {
                let s = f32::from_le_bytes(bytes);
                // Backpressure: wait until the audio buffer has space rather than
                // dropping samples (which would cause drift).
                loop {
                    if audio_prod.push(s).is_ok() { break; }
                    thread::sleep(Duration::from_micros(500));
                }
            }
            Err(_) => break,
        }
    }

    let _ = ffmpeg.wait();
    let _ = std::fs::remove_file(&downloaded);
    eprintln!("[url] stream ended");
    Ok(())
}

fn download(url: &str) -> anyhow::Result<PathBuf> {
    let tmp_dir = std::env::temp_dir();
    let stem = format!("audio_viz_{}", std::process::id());
    let template = tmp_dir.join(format!("{}.%(ext)s", stem));

    eprintln!("[url] downloading…");
    let status = Command::new("yt-dlp")
        .args([
            "-f", "bestaudio[ext=m4a]/bestaudio[ext=mp4]/bestaudio",
            "--no-playlist",
            "-o", template.to_str().unwrap(),
            url,
        ])
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run yt-dlp — install with: brew install yt-dlp")?;

    anyhow::ensure!(status.success(), "yt-dlp exited with an error");
    find_file(&tmp_dir, &stem)
}

fn find_file(dir: &Path, stem: &str) -> anyhow::Result<PathBuf> {
    std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(stem))
        .map(|e| e.path())
        .context("Could not find the downloaded audio file")
}

// ── Custom rodio Source ───────────────────────────────────────────────────────

/// Pulls samples from the audio ring buffer for rodio playback.
/// Every sample that gets played is also forwarded to viz_prod so the
/// visualizer sees exactly what's being heard — guaranteed sync.
struct RingSource {
    consumer: HeapConsumer<f32>,
    viz_prod: HeapProducer<f32>,
}

impl RingSource {
    fn new(consumer: HeapConsumer<f32>, viz_prod: HeapProducer<f32>) -> Self {
        Self { consumer, viz_prod }
    }
}

impl Iterator for RingSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let s = self.consumer.pop().unwrap_or(0.0);
        let _ = self.viz_prod.push(s);  // forward to viz at playback time
        Some(s)
    }
}

impl Source for RingSource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { OUT_SAMPLE_RATE }
    fn total_duration(&self) -> Option<Duration> { None }
}
