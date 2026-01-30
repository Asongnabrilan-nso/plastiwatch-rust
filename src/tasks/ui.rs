// PlastiWatch V2 — UI Task
//
// Owns the OLED display, haptic motor, and button input manager.
// Polls the button at ~100 Hz and processes UI events from the AI and power
// tasks.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use esp_idf_hal::gpio::{AnyInputPin, AnyOutputPin, Input, Output, PinDriver};

use crate::config::*;
use crate::drivers::display::{OledDisplay, SharedBus};
use crate::drivers::haptic::HapticDriver;
use crate::events::{ActivityClass, UiEvent};
use crate::input::InputManager;

pub fn ui_task(
    bus: SharedBus,
    button_pin: PinDriver<'static, AnyInputPin, Input>,
    haptic_pin: PinDriver<'static, AnyOutputPin, Output>,
    ui_rx: Receiver<UiEvent>,
    ui_tx: Sender<UiEvent>,
    sleep_requested: Arc<AtomicBool>,
    last_activity_ms: Arc<AtomicU32>,
) {
    log::info!("UI task started");

    let mut display = OledDisplay::new(bus);
    let mut haptic = HapticDriver::new(haptic_pin);
    let mut input = InputManager::new(button_pin, ui_tx);

    // Start on the default UI (logo + PlastiBytes text).
    let mut showing_logo = true;
    let mut current_activity = ActivityClass::default();
    let mut current_battery: f32 = 100.0;

    if let Err(e) = display.show_default_ui() {
        log::error!("Display error: {}", e);
    }

    let poll_interval = Duration::from_millis(UI_POLL_INTERVAL_MS);

    loop {
        // 1. Poll the button (handles debounce + click detection internally).
        input.update();

        // 2. Drain all pending UI events (non-blocking).
        while let Ok(event) = ui_rx.try_recv() {
            match event {
                UiEvent::UpdateActivity(activity) => {
                    current_activity = activity;
                    if !showing_logo {
                        let _ = display.show_activity(current_activity, current_battery);
                    }
                }

                UiEvent::UpdateBattery(level) => {
                    current_battery = level;
                    if !showing_logo {
                        let _ = display.show_activity(current_activity, current_battery);
                    }
                }

                UiEvent::ButtonSingleClick => {
                    haptic.trigger();
                    last_activity_ms.store(crate::now_ms(), Ordering::Relaxed);

                    // Toggle between default UI and activity screen.
                    showing_logo = !showing_logo;
                    if showing_logo {
                        let _ = display.show_default_ui();
                    } else {
                        let _ = display.show_activity(current_activity, current_battery);
                    }
                }

                UiEvent::ButtonDoubleClick => {
                    haptic.trigger();
                    last_activity_ms.store(crate::now_ms(), Ordering::Relaxed);

                    // Force activity display.
                    showing_logo = false;
                    let _ = display.show_activity(current_activity, current_battery);
                }

                UiEvent::ButtonLongPress => {
                    // 3-second hold → power off.
                    haptic.buzz(Duration::from_millis(500));
                    let _ = display.turn_off();
                    sleep_requested.store(true, Ordering::SeqCst);
                    log::info!("Long press detected — requesting deep sleep");
                }
            }
        }

        // 3. If sleep was requested, stop refreshing (power task handles sleep entry).
        if sleep_requested.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        thread::sleep(poll_interval);
    }
}
