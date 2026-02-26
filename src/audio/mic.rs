use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use std::collections::VecDeque;

pub struct MicCapture {
    _stream: cpal::Stream, // must stay alive or audio stops
    consumer: HeapConsumer<f32>,
    window: VecDeque<f32>, // sliding window of the latest `fft_size` samples
    pub sample_rate: u32,
}

impl MicCapture {
    pub fn start(fft_size: usize) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No microphone input device found"))?;

        let supported = device.default_input_config()?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        // ring buffer: 8x fft_size so the callback never stalls
        let rb = HeapRb::<f32>::new(fft_size * 8);
        let (producer, consumer) = rb.split();

        let stream = build_stream(&device, &config, format, channels, producer)?;
        stream.play()?;

        Ok(Self {
            _stream: stream,
            consumer,
            window: VecDeque::from(vec![0.0f32; fft_size]),
            sample_rate,
        })
    }

    /// Drains new samples from the ring buffer into the sliding window,
    /// then copies the latest `size` samples into `out`.
    pub fn read_window(&mut self, out: &mut Vec<f32>, size: usize) {
        while let Some(s) = self.consumer.pop() {
            self.window.push_back(s);
            if self.window.len() > size {
                self.window.pop_front();
            }
        }
        out.clear();
        out.extend(self.window.iter().copied());
        // pad with silence if not enough samples yet (startup)
        while out.len() < size {
            out.push(0.0);
        }
    }
}

fn mic_err(e: cpal::StreamError) {
    eprintln!("Mic stream error: {e}");
}

/// Builds an input stream for the given sample format.
/// `producer` is moved into exactly one callback closure.
fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    channels: usize,
    mut producer: HeapProducer<f32>,
) -> anyhow::Result<cpal::Stream> {
    Ok(match format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _| {
                for chunk in data.chunks(channels) {
                    let mono = chunk.iter().sum::<f32>() / channels as f32;
                    let _ = producer.push(mono);
                }
            },
            mic_err,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _| {
                for chunk in data.chunks(channels) {
                    let mono = chunk
                        .iter()
                        .map(|&s| s as f32 / i16::MAX as f32)
                        .sum::<f32>()
                        / channels as f32;
                    let _ = producer.push(mono);
                }
            },
            mic_err,
            None,
        )?,
        _ => anyhow::bail!("Unsupported mic sample format: {:?}", format),
    })
}
