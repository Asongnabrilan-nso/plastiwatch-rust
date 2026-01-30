// PlastiWatch V2 — Button Input Manager
//
// Debounced button handler with single-click, double-click, and long-press
// detection.  Designed to be polled at ~100 Hz from the UI task.

use std::sync::mpsc::Sender;
use std::time::Instant;

use esp_idf_hal::gpio::{AnyInputPin, Input, PinDriver};

use crate::config::*;
use crate::events::UiEvent;

pub struct InputManager<'d> {
    pin: PinDriver<'d, AnyInputPin, Input>,
    ui_tx: Sender<UiEvent>,

    // Debounce state
    last_raw: bool,
    last_debounce: Instant,

    // Press tracking
    press_start: Option<Instant>,
    button_down: bool,

    // Double-click state machine
    waiting_for_second_click: bool,
    first_click_time: Instant,
}

impl<'d> InputManager<'d> {
    pub fn new(pin: PinDriver<'d, AnyInputPin, Input>, ui_tx: Sender<UiEvent>) -> Self {
        let now = Instant::now();
        Self {
            pin,
            ui_tx,
            last_raw: true, // pull-up → idle HIGH
            last_debounce: now,
            press_start: None,
            button_down: false,
            waiting_for_second_click: false,
            first_click_time: now,
        }
    }

    /// Call every ~10 ms from the UI task loop.
    pub fn update(&mut self) {
        let current = self.pin.is_high(); // true = released (pull-up)
        let now = Instant::now();

        // ---- debounce filter ----
        if current != self.last_raw {
            self.last_debounce = now;
        }
        self.last_raw = current;

        let stable_ms = now.duration_since(self.last_debounce).as_millis() as u64;
        if stable_ms < DEBOUNCE_MS {
            // Signal still bouncing — wait.
            self.check_double_click_timeout(now);
            return;
        }

        let pressed = !current; // active LOW

        // ---- button pressed edge ----
        if pressed && !self.button_down {
            self.button_down = true;
            self.press_start = Some(now);
        }

        // ---- button released edge ----
        if !pressed && self.button_down {
            self.button_down = false;
            let hold_ms = self
                .press_start
                .map(|t| now.duration_since(t).as_millis() as u64)
                .unwrap_or(0);

            if hold_ms >= LONG_PRESS_MS {
                let _ = self.ui_tx.send(UiEvent::ButtonLongPress);
                self.waiting_for_second_click = false;
            } else if self.waiting_for_second_click {
                // Second click within window → double-click
                let _ = self.ui_tx.send(UiEvent::ButtonDoubleClick);
                self.waiting_for_second_click = false;
            } else {
                // First short click — start double-click window
                self.waiting_for_second_click = true;
                self.first_click_time = now;
            }
        }

        self.check_double_click_timeout(now);
    }

    /// If the double-click window expires, emit a single-click.
    fn check_double_click_timeout(&mut self, now: Instant) {
        if self.waiting_for_second_click {
            let elapsed = now.duration_since(self.first_click_time).as_millis() as u64;
            if elapsed > DOUBLE_CLICK_WINDOW_MS {
                let _ = self.ui_tx.send(UiEvent::ButtonSingleClick);
                self.waiting_for_second_click = false;
            }
        }
    }
}
