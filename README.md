# audio-visualizer

A real-time audio visualizer written in Rust. Supports microphone input, local WAV files, and streaming audio from YouTube (or any site yt-dlp supports).



<video src="https://github.com/user-attachments/assets/c334f4db-24bd-438e-b593-aa7b64c6e371" width="700" autoplay loop muted playsinline></video>








## Features

- **Radial bar visualizer** — 120 frequency bars radiating outward, neon rainbow colors that shift with the bass
- **Waveform ring** — smooth 128-point waveform orbiting the center circle
- **Spectrum bar** — linear frequency spectrum along the bottom
- **Bass-reactive pulse** — the entire visualizer pulses and shifts color with kick/bass hits
- **Three audio modes** — mic, WAV file, or YouTube URL

## Requirements

- [Rust](https://rustup.rs/)
- [ffmpeg](https://ffmpeg.org/) — audio decoding
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — for URL mode only

```sh
brew install ffmpeg yt-dlp
```

## Usage

### Microphone (default)

```sh
cargo run
# or explicitly
cargo run -- mic
```

### WAV file

```sh
cargo run -- wav path/to/song.wav
# defaults to assets/song.wav if no path given
cargo run -- wav
```

### YouTube / URL

```sh
cargo run -- url "https://www.youtube.com/watch?v=..."
```

Any URL supported by yt-dlp works — YouTube, SoundCloud, etc. The audio downloads to a temp file, then streams through ffmpeg in real time. Visualization is synced directly to playback (not to the decode buffer), so audio and visuals are always in lockstep. Audio stops the moment you close the window.

## How it works

```
Mic     ─────────────────────────────────────────┐
WAV     ── hound decode ──────────────────────────┤
URL     ── yt-dlp → ffmpeg → ring buffer          │
                             ↓                    │
                         RingSource               │
                         (rodio playback)          │
                             ↓                    │
                         viz ring buffer           │
                             ↓                    ↓
                         FFT (2048-point, Hann window)
                             ↓
                         120 log-frequency bands
                             ↓
                         nannou render
```

**Sync architecture (URL mode):** A single ffmpeg process feeds one ring buffer. The custom `RingSource` rodio source plays each sample and simultaneously forwards it to a second viz-only ring buffer. The visualizer reads from this viz buffer, meaning it sees exactly the samples being played — zero drift possible.

## Tech stack

| Crate                                          | Role                               |
| ---------------------------------------------- | ---------------------------------- |
| [nannou](https://nannou.cc/)                   | windowing + 2D drawing             |
| [rustfft](https://github.com/ejmahler/RustFFT) | FFT computation                    |
| [cpal](https://github.com/RustAudio/cpal)      | microphone capture                 |
| [rodio](https://github.com/RustAudio/rodio)    | audio playback                     |
| [ringbuf](https://github.com/agerasev/ringbuf) | lock-free ring buffers             |
| [hound](https://github.com/ruuda/hound)        | WAV decoding                       |
| [clap](https://github.com/clap-rs/clap)        | CLI argument parsing               |
| yt-dlp + ffmpeg                                | audio download + decode (URL mode) |
