use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Zoom as ZoomView},
    storage,
    ui::{self, ButtonWidget, Widget},
};

const KEY: &str = "zk";
const KEY_LABEL: &str = "Zoom key (use the button instead)";
const LEVELS: [u8; 7] = [2, 3, 4, 5, 6, 8, 10];
const MAXES: [u8; 5] = [3, 4, 6, 8, 10];
const MIN: f32 = 1.1;
const STEP: f32 = 1.15;
const SPEED: f32 = 12.0;
const KEY_BUTTON: u32 = 2;
const LEVEL_BUTTON: u32 = 3;
const MAX_BUTTON: u32 = 4;
const SMOOTH_BUTTON: u32 = 5;
const SENSITIVITY_BUTTON: u32 = 6;

fn key_name(letter: u8) -> String {
    format!("key.keyboard.{}", letter as char)
}

fn next_letter(letter: u8) -> u8 {
    if letter >= b'z' || letter < b'a' { b'a' } else { letter + 1 }
}

struct FarZoom {
    letter: u8,
    level: u8,
    max: u8,
    smooth: bool,
    sensitivity: bool,
    held: bool,
    target: f32,
    shown: f32,
}

impl FarZoom {
    fn show(&self) {
        let factor = self.shown;
        render::set_zoom(ZoomView {
            factor,
            smooth: self.smooth,
            turn: if self.sensitivity && factor > 1.0 { 1.0 / factor } else { 1.0 },
        });
    }
    fn page(&self) {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        ui::set_settings(&[
            Widget::Button(ButtonWidget {
                id: KEY_BUTTON,
                label: format!("Zoom key: {}", (self.letter as char).to_ascii_uppercase()),
            }),
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
        let _ = storage::set("letter", &[self.letter]);
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
        let letter = stored("letter")
            .and_then(|v| v.trim().bytes().next())
            .map(|b| b.to_ascii_lowercase())
            .filter(|b| b.is_ascii_lowercase())
            .unwrap_or(b'c');
        input::register_key(KEY, KEY_LABEL, &key_name(letter))?;
        let max = stored("max")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .map_or(10, |v| v.clamp(2, 10));
        let level = stored("level")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .map_or(2, |v| v.clamp(2, max));
        let flag = |key: &str| stored(key).is_none_or(|v| v.trim() != "false");
        let zoom = FarZoom {
            letter,
            level,
            max,
            smooth: flag("smooth"),
            sensitivity: flag("sensitivity"),
            held: false,
            target: 1.0,
            shown: 1.0,
        };
        zoom.page();
        Ok((zoom, Registration::new().on_frame()))
    }
    fn frame(&mut self, seconds: f32) {
        let mut changed = false;
        for event in input::poll() {
            match event {
                Event::KeyDown(k) if k == KEY => {
                    self.held = true;
                    self.target = f32::from(self.level).clamp(MIN, f32::from(self.max));
                    self.shown = self.target;
                    input::claim_wheel(true);
                    changed = true;
                }
                Event::KeyUp(k) if k == KEY => {
                    self.held = false;
                    self.target = 1.0;
                    self.shown = 1.0;
                    input::claim_wheel(false);
                    changed = true;
                }
                Event::Wheel(steps) => {
                    if self.held {
                        let max = f32::from(self.max);
                        self.target = (self.target * STEP.powf(steps)).clamp(MIN, max);
                    }
                }
                _ => {}
            }
        }
        if self.held && self.shown != self.target {
            let k = (seconds * SPEED).clamp(0.0, 1.0);
            let next = self.shown + (self.target - self.shown) * k;
            if (self.target - next).abs() < 0.005 {
                self.shown = self.target;
            } else {
                self.shown = next;
            }
            changed = true;
        }
        if changed {
            self.show();
        }
    }
    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            KEY_BUTTON => {
                self.letter = next_letter(self.letter);
                if self.held {
                    self.held = false;
                    self.target = 1.0;
                    self.shown = 1.0;
                    input::claim_wheel(false);
                    self.show();
                }
                let _ = input::register_key(KEY, KEY_LABEL, &key_name(self.letter));
            }
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
