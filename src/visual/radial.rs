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
            base_radius: 100.0,
            bar_gain: 180.0,

            hue_base: 0.58,
            hue_range: 0.20,
            pulse_gain: 60.0,

            fade_alpha: 0.12,
        }
    }

    pub fn draw(&self, app: &App, frame: Frame, bands: &[f32], bass_fast: f32, bass_smooth: f32) {
        let draw = app.draw();

        //trails
        draw.rect()
            .wh(app.window_rect().wh())
            .color(srgba(0.0, 0.0, 0.0, self.fade_alpha));

        let hue = (self.hue_base + bass_smooth * self.hue_range).fract();
        let radius = self.base_radius + bass_fast * self.pulse_gain;

        //glow intensity
        let glow = (0.15 + bass_smooth * 0.35).clamp(0.12, 0.55);

        let B = self.bars as f32;
        for (i, &v) in bands.iter().take(self.bars).enumerate() {
            let theta = (i as f32 / B) * TAU;
            let dir = vec2(theta.cos(), theta.sin());

            let len = (v * self.bar_gain).clamp(0.0, 520.0);
            let p0 = dir * radius;
            let p1 = dir * (radius + len);

            let h = (hue + (i as f32 / B) * 0.08).fract();

            //outer glow
            draw.line()
                .start(p0)
                .end(p1)
                .weight(6.0)
                .color(hsva(h, 1.0, 1.0, glow));

            //bright core
            draw.line()
                .start(p0)
                .end(p1)
                .weight(2.0)
                .color(hsva(h, 1.0, 1.0, 0.9));
        }
        draw.to_frame(app, &frame).unwrap();
    }
}
