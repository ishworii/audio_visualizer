use hound::{SampleFormat, WavReader};
use std::{i16, path::Path};

#[derive(Clone)]
pub struct AudioData {
    pub sample_rate: u32,
    pub samples_mono: Vec<f32>,
    pub duration_sec: f32,
}

impl AudioData {
    pub fn load_wav<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();
        if spec.sample_format != SampleFormat::Int {
            anyhow::bail!("Only PCM integer WAV supported for now.");
        }

        let sample_rate = spec.sample_rate;
        let channels = spec.channels as usize;
        if channels != 1 && channels != 2 {
            anyhow::bail!("Only mono/stereo WAV supported got({channels}) instead.");
        }
        //only hanldle 16-bit PCM for now
        if spec.bits_per_sample != 16 {
            anyhow::bail!(
                "Only 16-bit PCM WAV supported for now got ({}-bit)",
                spec.bits_per_sample
            );
        }

        let mut mono = Vec::new();

        //read frames
        let mut frame = Vec::with_capacity(channels);
        for s in reader.samples::<i16>() {
            let s = s? as f32 / i16::MAX as f32;
            frame.push(s);
            if frame.len() == channels {
                let m = if channels == 1 {
                    frame[0]
                } else {
                    (frame[0] + frame[1]) * 0.5
                };
                mono.push(m);
                frame.clear();
            }
        }
        let duration_sec = mono.len() as f32 / sample_rate as f32;
        Ok(Self {
            sample_rate,
            samples_mono: mono,
            duration_sec,
        })
    }

    //returns a centered window of n samples at time t_sec
    pub fn window_at_time(&self, t_sec: f32, n: usize, out: &mut Vec<f32>) {
        out.clear();
        out.reserve(n);

        let dur = self.duration_sec.max(0.000_1);
        let t = t_sec.rem_euclid(dur);

        let center = (t * self.sample_rate as f32) as isize;
        let half = (n as isize) / 2;

        let len = self.samples_mono.len() as isize;

        for i in 0..(n as isize) {
            let idx = (center - half + i).rem_euclid(len) as usize;
            out.push(self.samples_mono[idx]);
        }
    }
}
