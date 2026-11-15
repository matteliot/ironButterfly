//! Spoken announcements: prompts embedded as 16 kHz G.711 mu-law (made with
//! `tools/make-prompts.sh`), played at 48 kHz by linear interpolation, then
//! a 7 kHz low-pass, with a ducking envelope for the music under them.

use super::dsp::{OnePole, coef};

/// Sample rate of the embedded prompts.
pub const PROMPT_SR: f32 = 16_000.0;

/// G.711 mu-law byte to a sample in -1..1.
#[inline]
pub fn ulaw(b: u8) -> f32 {
    let u = !b;
    let exp = (u >> 4) & 7;
    let mag = ((((u & 0x0F) as i32) << 3) + 0x84) << exp;
    let s = (mag - 0x84) as f32 * (1.0 / 32768.0);
    if u & 0x80 != 0 { -s } else { s }
}

pub struct Announcer {
    prompts: &'static [&'static [u8]],
    cur: usize,
    pos: f32,
    pub speaking: bool,
    lp: OnePole,
    /// 0..1, follows `speaking`.
    pub duck: f32,
    c_duck: f32,
}

impl Announcer {
    pub const fn new(prompts: &'static [&'static [u8]]) -> Self {
        Self { prompts, cur: 0, pos: 0.0, speaking: false, lp: OnePole::NEW, duck: 0.0, c_duck: 1.0 }
    }

    /// The ducking envelope's time constant (s).
    pub fn init(&mut self, duck_s: f32) {
        self.lp.set(7000.0);
        self.c_duck = coef(duck_s);
    }

    /// (Re)starts prompt `i` from the top.
    pub fn say(&mut self, i: usize) {
        if i < self.prompts.len() {
            self.cur = i;
            self.pos = 0.0;
            self.speaking = true;
        }
    }

    /// The next sample of the current prompt (0 when done).
    #[inline]
    pub fn speak(&mut self) -> f32 {
        let mut v = 0.0;
        if self.speaking {
            let p = self.prompts[self.cur];
            let i = self.pos as usize;
            if i + 1 < p.len() {
                let fr = self.pos - i as f32;
                let (a, c) = (ulaw(p[i]), ulaw(p[i + 1]));
                v = a + (c - a) * fr;
                self.pos += PROMPT_SR / super::dsp::SR;
            } else {
                self.speaking = false;
            }
        }
        self.lp.process(v)
    }

    /// Steps the ducking envelope towards `speaking`; returns it.
    #[inline]
    pub fn duck_step(&mut self) -> f32 {
        let st = if self.speaking { 1.0 } else { 0.0 };
        self.duck += (st - self.duck) * self.c_duck;
        self.duck
    }
}
