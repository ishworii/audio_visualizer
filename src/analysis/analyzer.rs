use rustfft::{FftPlanner, num_complex::Complex32};

pub struct AnalysisFrame {
    pub bands: Vec<f32>,
    pub bass_fast: f32,
    pub bass_smooth: f32,
}

pub struct Analyzer {
    pub fft_size: usize,
    pub bars: usize,
    pub f_min: f32,
    pub f_max: f32,

    hann: Vec<f32>,
    fft_in: Vec<Complex32>,
    fft_out: Vec<Complex32>,
    magnitues: Vec<f32>,

    smoothed_bands: Vec<f32>,
    bass_fast: f32,
    bass_smooth: f32,

    alpha_bands: f32,
    alpha_bass_slow: f32,
    alpha_bass_fast: f32,

    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
}

impl Analyzer {
    pub fn new(sample_rate: u32, fft_size: usize, bars: usize) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let hann = (0..fft_size)
            .map(|n| {
                let n = n as f32;
                let n_max = (fft_size - 1) as f32;
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * n / n_max).cos())
            })
            .collect::<Vec<_>>();
        let half = fft_size / 2;
        let f_max = (sample_rate as f32 * 0.5).min(18_000.0);

        Self {
            fft_size,
            bars,
            f_min: 20.0,
            f_max,

            hann,
            fft_in: vec![Complex32::new(0.0, 0.0); fft_size],
            fft_out: vec![Complex32::new(0.0, 0.0); fft_size],
            magnitues: vec![0.0; half],

            smoothed_bands: vec![0.0; bars],
            bass_fast: 0.0,
            bass_smooth: 0.0,

            alpha_bands: 0.12,     //cinematic smooth
            alpha_bass_slow: 0.08, //color/glow
            alpha_bass_fast: 0.30, //pulse

            fft,
        }
    }
    pub fn analyze(&mut self, window: &[f32], sample_rate: u32) -> AnalysisFrame {
        debug_assert_eq!(window.len(), self.fft_size);

        //window + complex input
        for i in 0..self.fft_size {
            let x = window[i] * self.hann[i];
            self.fft_in[i] = Complex32::new(x, 0.0);
            self.fft_out[i] = self.fft_in[i];
        }

        //fft in-place and fft out
        self.fft.process(&mut self.fft_out);

        //magnitudes for 0..N/2, normalized by fft_size so values stay in ~[0,1]
        let half = self.fft_size / 2;
        let norm = 1.0 / (self.fft_size as f32 * 0.5);
        for i in 0..half {
            let c = self.fft_out[i];
            let mag = (c.re * c.re + c.im * c.im).sqrt() * norm;
            self.magnitues[i] = mag;
        }

        //bass 20 to 120hz from raw magnitudes
        let bass_raw = self.bass_energy_from_bins(sample_rate, 20.0, 120.0);

        //fast + smooth bass
        self.bass_fast += self.alpha_bass_fast * (bass_raw - self.bass_fast);
        self.bass_smooth += self.alpha_bass_slow * (bass_raw - self.bass_smooth);

        //log bands
        let mut bands = vec![0.0f32; self.bars];
        let r = self.f_max / self.f_min;

        for b in 0..self.bars {
            let t0 = b as f32 / self.bars as f32;
            let t1 = (b + 1) as f32 / self.bars as f32;
            let f0 = self.f_min * r.powf(t0);
            let f1 = self.f_min * r.powf(t1);

            let (i0, i1) = self.freq_range_to_bin_range(sample_rate, f0, f1);

            let mut sum = 0.0;
            let mut count = 0.0;
            for i in i0..i1 {
                sum += self.magnitues[i];
                count += 1.0;
            }
            let avg: f32 = if count > 0.0 { sum / count } else { 0.0 };

            bands[b] = avg.sqrt();
        }

        //smooth bands
        for b in 0..self.bars {
            self.smoothed_bands[b] += self.alpha_bands * (bands[b] - self.smoothed_bands[b]);
        }

        AnalysisFrame {
            bands: self.smoothed_bands.clone(),
            bass_fast: self.bass_fast,
            bass_smooth: self.bass_smooth,
        }
    }
    fn freq_range_to_bin_range(&self, sample_rate: u32, f0: f32, f1: f32) -> (usize, usize) {
        let sr = sample_rate as f32;
        let n = self.fft_size as f32;
        let half = self.fft_size / 2;

        let mut i0 = (f0 * n / sr).floor() as isize;
        let mut i1 = (f1 * n / sr).ceil() as isize;

        if i0 < 0 {
            i0 = 0;
        }
        if i1 < 0 {
            i1 = 0;
        }

        let mut i0 = i0 as usize;
        let mut i1 = i1 as usize;

        if i0 >= half {
            i0 = half - 1;
        }
        if i1 > half {
            i1 = half;
        }
        if i1 <= i0 {
            i1 = (i0 + 1).min(half);
        }

        (i0, i1)
    }

    fn bass_energy_from_bins(&self, sample_rate: u32, f0: f32, f1: f32) -> f32 {
        let (i0, i1) = self.freq_range_to_bin_range(sample_rate, f0, f1);
        let mut sum = 0.0;
        let mut count = 0.0;
        for i in i0..i1 {
            sum += self.magnitues[i];
            count += 1.0;
        }
        let avg = if count > 0.0 { sum / count } else { 0.0 };
        avg.sqrt()
    }
}
