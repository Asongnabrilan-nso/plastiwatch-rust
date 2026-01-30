// PlastiWatch V2 — Haptic Motor Driver
//
// Simple GPIO-driven vibration motor.

use std::thread;
use std::time::Duration;

use esp_idf_hal::gpio::{Output, PinDriver};

pub struct HapticDriver<'d> {
    pin: PinDriver<'d, esp_idf_hal::gpio::AnyOutputPin, Output>,
}

impl<'d> HapticDriver<'d> {
    pub fn new(pin: PinDriver<'d, esp_idf_hal::gpio::AnyOutputPin, Output>) -> Self {
        Self { pin }
    }

    /// Short 50 ms vibration pulse — tactile feedback for button clicks.
    pub fn trigger(&mut self) {
        self.buzz(Duration::from_millis(50));
    }

    /// Vibrate for a custom duration (blocks the calling thread).
    pub fn buzz(&mut self, duration: Duration) {
        let _ = self.pin.set_high();
        thread::sleep(duration);
        let _ = self.pin.set_low();
    }
}
