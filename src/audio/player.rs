use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

pub struct AudioPlayer {
    _stream: OutputStream, // must stay alive for audio to keep playing
    start: Instant,
}

impl AudioPlayer {
    pub fn start(path: &str) -> Self {
        let (_stream, handle) =
            OutputStream::try_default().expect("Failed to open audio output device");
        let sink = Sink::try_new(&handle).expect("Failed to create audio sink");
        let file = BufReader::new(File::open(path).expect("Failed to open WAV for playback"));
        let source = Decoder::new(file).expect("Failed to decode WAV for playback");
        sink.append(source.repeat_infinite());
        sink.detach();
        Self {
            _stream,
            start: Instant::now(),
        }
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }
}
