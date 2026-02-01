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
// use crate::drivers::sprites::{AnimationState, get_frame_count};
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
    
    // Animation state for each activity type
    let mut animation_states: [AnimationState; 4] = [
        AnimationState::new(get_frame_count(ActivityClass::Idle)),
        AnimationState::new(get_frame_count(ActivityClass::Snake)),
        AnimationState::new(get_frame_count(ActivityClass::UpDown)),
        AnimationState::new(get_frame_count(ActivityClass::Wave)),
    ];
    let mut current_animation = &mut animation_states[0]; // Start with Idle

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
                    // Switch to the appropriate animation state
                    current_animation = match activity {
                        ActivityClass::Idle => &mut animation_states[0],
                        ActivityClass::Snake => &mut animation_states[1],
                        ActivityClass::UpDown => &mut animation_states[2],
                        ActivityClass::Wave => &mut animation_states[3],
                    };
                    current_animation.reset();
                    if !showing_logo {
                        let _ = display.show_activity(current_activity, current_battery, current_animation);
                    }
                }

                UiEvent::UpdateBattery(level) => {
                    current_battery = level;
                    if !showing_logo {
                        let _ = display.show_activity(current_activity, current_battery, current_animation);
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
                        let _ = display.show_activity(current_activity, current_battery, current_animation);
                    }
                }

                UiEvent::ButtonDoubleClick => {
                    haptic.trigger();
                    last_activity_ms.store(crate::now_ms(), Ordering::Relaxed);

                    // Force activity display.
                    showing_logo = false;
                    let _ = display.show_activity(current_activity, current_battery, current_animation);
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

        // 3. Update animation if on activity screen
        if !showing_logo {
            let current_ms = crate::now_ms();
            if current_animation.update(current_ms) {
                // Frame changed, redraw
                let _ = display.show_activity(current_activity, current_battery, current_animation);
            }
        }

        // 4. If sleep was requested, stop refreshing (power task handles sleep entry).
        if sleep_requested.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        thread::sleep(poll_interval);
    }
}
