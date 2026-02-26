use crate::{analysis::Analyzer, audio::AudioData, visual::RadialVisualizer};

mod analysis;
mod audio;
mod visual;

use nannou::prelude::*;

const FFT_SIZE: usize = 2048;
const BARS: usize = 120;

struct Model {
    audio: AudioData,
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
    app.new_window()
        .size(1000, 1000)
        .view(view)
        .build()
        .unwrap();
    let wav_path = "assets/song.wav";

    let audio = AudioData::load_wav(wav_path).expect("Failed to load WAV...");

    let analyzer = Analyzer::new(audio.sample_rate, FFT_SIZE, BARS);
    let visual = RadialVisualizer::new(BARS);

    Model {
        audio,
        analyzer,
        visual,

        scratch_window: Vec::with_capacity(FFT_SIZE),
        latest_bands: vec![0.0; BARS],
        bass_fast: 0.0,
        bass_smooth: 0.0,
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let t = app.time;

    model
        .audio
        .window_at_time(t, FFT_SIZE, &mut model.scratch_window);

    let frame = model
        .analyzer
        .analyze(&model.scratch_window, model.audio.sample_rate);
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
    );
}
