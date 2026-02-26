mod analysis;
mod audio;
mod visual;

use analysis::Analyzer;
use audio::{AudioData, AudioPlayer, MicCapture, UrlStream};
use clap::{Parser, Subcommand};
use nannou::prelude::*;
use visual::RadialVisualizer;

const FFT_SIZE: usize = 2048;
const BARS: usize = 120;
const DEFAULT_WAV: &str = "assets/song.wav";

#[derive(Parser)]
#[command(about = "Audio visualizer — mic, WAV file, or YouTube URL")]
struct Cli {
    #[command(subcommand)]
    mode: Option<Mode>,
}

#[derive(Subcommand)]
enum Mode {
    /// Visualize microphone input (default when no subcommand given)
    Mic,
    /// Visualize a WAV file
    Wav {
        /// Path to the WAV file
        #[arg(default_value = DEFAULT_WAV)]
        file: String,
    },
    /// Visualize audio from a YouTube (or any yt-dlp-supported) URL
    /// Requires: brew install yt-dlp ffmpeg
    Url {
        /// URL to stream audio from
        url: String,
    },
}

enum AudioSource {
    Mic(MicCapture),
    Wav { audio: AudioData, player: AudioPlayer },
    Url(UrlStream),
}

impl AudioSource {
    fn fill_window(&mut self, scratch: &mut Vec<f32>, fft_size: usize) {
        match self {
            Self::Mic(mic) => mic.read_window(scratch, fft_size),
            Self::Wav { audio, player } => {
                audio.window_at_time(player.elapsed_secs(), fft_size, scratch)
            }
            Self::Url(stream) => stream.read_window(scratch, fft_size),
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            Self::Mic(mic) => mic.sample_rate,
            Self::Wav { audio, .. } => audio.sample_rate,
            Self::Url(stream) => stream.sample_rate,
        }
    }
}


struct Model {
    source: AudioSource,
    analyzer: Analyzer,
    visual: RadialVisualizer,
    scratch_window: Vec<f32>,
    latest_bands: Vec<f32>,
    bass_fast: f32,
    bass_smooth: f32,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    app.new_window().size(800, 800).view(view).build().unwrap();

    let cli = Cli::parse();
    let source = match cli.mode.unwrap_or(Mode::Mic) {
        Mode::Mic => {
            let mic = MicCapture::start(FFT_SIZE).expect("Failed to start mic capture");
            AudioSource::Mic(mic)
        }
        Mode::Wav { file } => {
            let audio = AudioData::load_wav(&file).expect("Failed to load WAV");
            let player = AudioPlayer::start(&file);
            AudioSource::Wav { audio, player }
        }
        Mode::Url { url } => {
            let stream = UrlStream::start(&url, FFT_SIZE)
                .expect("Failed to start URL stream — is yt-dlp and ffmpeg installed?");
            AudioSource::Url(stream)
        }
    };

    let analyzer = Analyzer::new(source.sample_rate(), FFT_SIZE, BARS);
    let visual = RadialVisualizer::new(BARS);

    Model {
        source,
        analyzer,
        visual,
        scratch_window: Vec::with_capacity(FFT_SIZE),
        latest_bands: vec![0.0; BARS],
        bass_fast: 0.0,
        bass_smooth: 0.0,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    model
        .source
        .fill_window(&mut model.scratch_window, FFT_SIZE);
    let frame = model
        .analyzer
        .analyze(&model.scratch_window, model.source.sample_rate());
    model.latest_bands = frame.bands;
    model.bass_fast = frame.bass_fast;
    model.bass_smooth = frame.bass_smooth;
}

fn view(app: &App, model: &Model, frame: Frame) {
    model.visual.draw(
        app,
        frame,
        &model.latest_bands,
        model.bass_fast,
        model.bass_smooth,
        &model.scratch_window,
    );
}
