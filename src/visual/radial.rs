use nannou::prelude::*;

pub struct RadialVisualizer {
    pub bars: usize,
    pub base_radius: f32,
    pub bar_gain: f32,

    pub hue_base: f32,
    pub hue_range: f32,
    pub pulse_gain: f32,

    pub fade_alpha: f32,
}

impl RadialVisualizer {
    pub fn new(bars: usize) -> Self {
        Self {
            bars,
            base_radius: 150.0,
            bar_gain: 400.0,

            hue_base: 0.0,
            hue_range: 0.30,
            pulse_gain: 60.0,

            fade_alpha: 0.12,
        }
    }

    pub fn draw(
        &self,
        app: &App,
        frame: Frame,
        bands: &[f32],
        bass_fast: f32,
        bass_smooth: f32,
        waveform: &[f32],
    ) {
        let draw = app.draw();
        let win = app.window_rect();

        let hue = (self.hue_base + bass_smooth * self.hue_range).fract();
        let radius = (self.base_radius + bass_fast * self.pulse_gain).clamp(0.0, 350.0);
        let glow = (0.15 + bass_smooth * 0.35).clamp(0.12, 0.55);

        // 1. background fade
        draw.rect()
            .wh(win.wh())
            .color(srgba(0.0, 0.0, 0.0, self.fade_alpha));

        // 2. waveform ring — average blocks of samples so the shape is smooth, not noisy
        if !waveform.is_empty() {
            let wf_radius = self.base_radius * 0.65;
            let wf_gain = 35.0;
            let n = 128;
            let chunk = (waveform.len() / n).max(1);

            let mut ring_pts: Vec<Point2> = (0..n)
                .map(|i| {
                    let start = i * chunk;
                    let end = (start + chunk).min(waveform.len());
                    let avg = waveform[start..end].iter().sum::<f32>() / (end - start) as f32;
                    let r = wf_radius + avg * wf_gain;
                    let theta = (i as f32 / n as f32) * TAU;
                    pt2(theta.cos() * r, theta.sin() * r)
                })
                .collect();

            // close the loop
            if let Some(&first) = ring_pts.first() {
                ring_pts.push(first);
            }

            draw.polyline()
                .weight(1.5)
                .points(ring_pts)
                .color(hsva(hue, 0.7, 1.0, 0.55));
        }

        // 3. radial bars
        let bars_f = self.bars as f32;
        for (i, &v) in bands.iter().take(self.bars).enumerate() {
            let theta = (i as f32 / bars_f) * TAU;
            let dir = vec2(theta.cos(), theta.sin());

            let len = (v * self.bar_gain).clamp(0.0, 480.0 - radius);
            let p0 = dir * radius;
            let p1 = dir * (radius + len);

            let h = (hue + (i as f32 / bars_f) * 1.0).fract();

            // outer glow
            draw.line()
                .start(p0)
                .end(p1)
                .weight(6.0)
                .color(hsva(h, 1.0, 1.0, glow));

            // bright core
            draw.line()
                .start(p0)
                .end(p1)
                .weight(2.0)
                .color(hsva(h, 1.0, 1.0, 0.9));
        }

        // 4. bottom spectrum bar
        let bar_w = win.w() / bars_f;
        let base_y = win.bottom() + 2.0;
        for (i, &v) in bands.iter().take(self.bars).enumerate() {
            let bar_h = (v * self.bar_gain * 0.65).clamp(0.0, 260.0);
            if bar_h < 1.0 {
                continue;
            }
            let x = win.left() + (i as f32 + 0.5) * bar_w;
            let h = (hue + (i as f32 / bars_f) * 1.0).fract();

            // glow layer
            draw.rect()
                .x_y(x, base_y + bar_h * 0.5)
                .w_h(bar_w - 1.0, bar_h)
                .color(hsva(h, 1.0, 1.0, glow));

            // bright core
            draw.rect()
                .x_y(x, base_y + bar_h * 0.5)
                .w_h((bar_w - 1.0) * 0.4, bar_h)
                .color(hsva(h, 1.0, 1.0, 0.9));
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
