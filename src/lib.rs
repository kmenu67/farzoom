use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Zoom as ZoomView},
    storage,
    ui::{self, ButtonWidget, Widget},
};

const KEY: &str = "zoom";
const LEVELS: [u8; 7] = [2, 3, 4, 5, 6, 8, 10];
const MAXES: [u8; 5] = [3, 4, 6, 8, 10];
const MIN: f32 = 1.25;
const STEP: f32 = 1.25;
const LEVEL_BUTTON: u32 = 2;
const MAX_BUTTON: u32 = 3;
const SMOOTH_BUTTON: u32 = 4;
const SENSITIVITY_BUTTON: u32 = 5;

struct FarZoom {
    level: u8,
    max: u8,
    smooth: bool,
    sensitivity: bool,
    held: Option<f32>,
}

impl FarZoom {
    fn show(&self) {
        let factor = self.held.unwrap_or(1.0);
        render::set_zoom(ZoomView {
            factor,
            smooth: self.smooth,
            turn: if self.sensitivity { 1.0 / factor } else { 1.0 },
        });
    }
    fn page(&self) {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        ui::set_settings(&[
            Widget::Button(ButtonWidget {
                id: LEVEL_BUTTON,
                label: format!("Starting zoom: {}x", self.level),
            }),
            Widget::Button(ButtonWidget {
                id: MAX_BUTTON,
                label: format!("Max zoom: {}x", self.max),
            }),
            Widget::Button(ButtonWidget {
                id: SMOOTH_BUTTON,
                label: format!("Smooth zoom: {}", on(self.smooth)),
            }),
            Widget::Button(ButtonWidget {
                id: SENSITIVITY_BUTTON,
                label: format!("Slower turning: {}", on(self.sensitivity)),
            }),
        ]);
    }
    fn save(&self) {
        let flag = |b: bool| if b { &b"true"[..] } else { &b"false"[..] };
        let _ = storage::set("level", self.level.to_string().as_bytes());
        let _ = storage::set("max", self.max.to_string().as_bytes());
        let _ = storage::set("smooth", flag(self.smooth));
        let _ = storage::set("sensitivity", flag(self.sensitivity));
    }
}

fn stored(key: &str) -> Option<String> {
    storage::get(key).and_then(|v| String::from_utf8(v).ok())
}

impl Mod for FarZoom {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, "Zoom key", "key.keyboard.c")?;
        let max = stored("max")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .map_or(10, |v| v.clamp(2, 10));
        let level = stored("level")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .map_or(2, |v| v.clamp(2, max));
        let flag = |key: &str| stored(key).is_none_or(|v| v.trim() != "false");
        let zoom = FarZoom {
            level,
            max,
            smooth: flag("smooth"),
            sensitivity: flag("sensitivity"),
            held: None,
        };
        zoom.page();
        Ok((zoom, Registration::new().on_frame()))
    }
    fn frame(&mut self, _seconds: f32) {
        let mut changed = false;
        for event in input::poll() {
            match event {
                Event::KeyDown(k) if k == KEY => {
                    self.held = Some(f32::from(self.level).clamp(MIN, f32::from(self.max)));
                    input::claim_wheel(true);
                }
                Event::KeyUp(k) if k == KEY => {
                    self.held = None;
                    input::claim_wheel(false);
                }
                Event::Wheel(steps) => {
                    let max = f32::from(self.max);
                    let Some(z) = &mut self.held else { continue };
                    *z = (*z * STEP.powf(steps)).clamp(MIN, max);
                }
                _ => continue,
            }
            changed = true;
        }
        if changed {
            self.show();
        }
    }
    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            LEVEL_BUTTON => {
                let i = LEVELS.iter().position(|&l| l == self.level).unwrap_or(0);
                self.level = LEVELS[(i + 1) % LEVELS.len()];
                if self.level > self.max {
                    self.level = LEVELS[0];
                }
            }
            MAX_BUTTON => {
                let i = MAXES.iter().position(|&m| m == self.max).unwrap_or(0);
                self.max = MAXES[(i + 1) % MAXES.len()];
                self.level = self.level.min(self.max);
            }
            SMOOTH_BUTTON => self.smooth = !self.smooth,
            SENSITIVITY_BUTTON => self.sensitivity = !self.sensitivity,
            _ => return,
        }
        self.save();
        self.page();
    }
    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(FarZoom);
