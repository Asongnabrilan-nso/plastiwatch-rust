// PlastiWatch V2 — Firmware Entry Point
//
// Boot sequence:
//   1. Wait for the user button to be held for 3 seconds (boot trigger).
//   2. Display the PlastiBytes logo for 1 second.
//   3. Display "PlastiWatch" text for 1 second.
//   4. Run component self-test (OLED + MPU6050).
//   5. Enter default UI (logo + "PlastiBytes" label).
//   6. Spawn sensor, AI, UI, and power tasks.
//
// The system enters deep sleep when:
//   - The user holds the button for 3 seconds (long-press).
//   - No activity is detected for 3 minutes.

mod config;
mod drivers;
mod ei;
mod events;
mod input;
mod tasks;

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use esp_idf_hal::gpio::{AnyInputPin, AnyOutputPin, IOPin, Input, InputPin, Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::prelude::*;

use crate::config::*;
use crate::drivers::display::OledDisplay;
use crate::drivers::imu::Mpu6050;

// ---------------------------------------------------------------------------
// Utility: milliseconds since boot (wraps at ~49 days — fine for timeouts)
// ---------------------------------------------------------------------------
pub fn now_ms() -> u32 {
    unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u32 }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
fn main() -> anyhow::Result<()> {
    // Link esp-idf-sys runtime patches and initialise logging.
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("PlastiWatch V2 firmware starting…");

    // ---- Peripherals ------------------------------------------------------
    let peripherals = Peripherals::take()?;

    // Button GPIO (pull-up, active LOW) — used first for boot-hold detection.
    let button = PinDriver::input(peripherals.pins.gpio3.downgrade_input())?;
    configure_pullup(&button);

    // ---- Boot trigger: hold button for 3 seconds --------------------------
    if !wait_for_boot_hold(&button) {
        log::info!("Boot trigger not met — entering deep sleep");
        enter_deep_sleep();
    }
    log::info!("Boot trigger confirmed");

    // ---- I2C bus (shared between OLED and MPU6050) ------------------------
    let i2c_config = I2cConfig::new().baudrate(400u32.kHz().into());
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio6, // SDA
        peripherals.pins.gpio7, // SCL
        &i2c_config,
    )?;
    // SAFETY: The I2C peripheral is a singleton obtained from `Peripherals::take()`.
    // It will live for the entire programme duration (embedded firmware never exits).
    let i2c_bus: &'static Mutex<I2cDriver<'static>> =
        Box::leak(Box::new(Mutex::new(unsafe { core::mem::transmute(i2c) })));

    // ---- Boot sequence (display) ------------------------------------------
    let mut display = OledDisplay::new(i2c_bus);
    display.init()?;

    // Step 1 — PlastiBytes logo splash
    display.show_logo()?;
    thread::sleep(Duration::from_millis(BOOT_LOGO_DISPLAY_MS));

    // Step 2 — "PlastiWatch" text splash
    display.show_centered_text("PlastiWatch")?;
    thread::sleep(Duration::from_millis(BOOT_TEXT_DISPLAY_MS));

    // Step 3 — Component self-test
    let oled_ok = display.is_connected();
    let imu = Mpu6050::new(i2c_bus);
    let imu_ok = imu.is_connected();

    display.show_boot_status(oled_ok, imu_ok)?;
    thread::sleep(Duration::from_secs(1));

    if !oled_ok || !imu_ok {
        log::error!("Boot check FAILED — OLED:{} IMU:{}", oled_ok, imu_ok);
        // Continue anyway so we can still debug via serial.
    }

    // Step 4 — Default UI
    display.show_default_ui()?;
    log::info!("Boot complete — entering normal operation");

    // ---- Channels ---------------------------------------------------------
    let (sensor_tx, sensor_rx) = mpsc::channel();
    let (ui_tx, ui_rx) = mpsc::channel();

    // ---- Shared state -----------------------------------------------------
    let sleep_requested = Arc::new(AtomicBool::new(false));
    let last_activity_ms = Arc::new(AtomicU32::new(now_ms()));

    // ---- Prepare GPIO handles for tasks -----------------------------------
    // Re-use the button PinDriver (already configured) — extend to 'static.
    // SAFETY: GPIO peripheral lives forever, same argument as I2C above.
    let button_static: PinDriver<'static, AnyInputPin, Input> =
        unsafe { core::mem::transmute(button) };

    let haptic_pin = PinDriver::output(peripherals.pins.gpio4.downgrade_output())?;
    let haptic_static: PinDriver<'static, AnyOutputPin, Output> =
        unsafe { core::mem::transmute(haptic_pin) };

    // ---- Spawn tasks (map to FreeRTOS tasks via std::thread) ---------------

    // Sensor task — highest effective priority (tightest timing).
    let sensor_bus = i2c_bus;
    thread::Builder::new()
        .name("sensor".into())
        .stack_size(STACK_SENSOR)
        .spawn(move || {
            tasks::sensor::sensor_task(sensor_bus, sensor_tx);
        })?;

    // AI inference task
    let ai_ui_tx = ui_tx.clone();
    let ai_activity = Arc::clone(&last_activity_ms);
    thread::Builder::new()
        .name("ai".into())
        .stack_size(STACK_AI)
        .spawn(move || {
            tasks::ai::ai_task(sensor_rx, ai_ui_tx, ai_activity);
        })?;

    // UI task (display + button + haptic)
    let ui_sleep = Arc::clone(&sleep_requested);
    let ui_activity = Arc::clone(&last_activity_ms);
    let ui_tx_for_input = ui_tx.clone();
    thread::Builder::new()
        .name("ui".into())
        .stack_size(STACK_UI)
        .spawn(move || {
            tasks::ui::ui_task(
                i2c_bus,
                button_static,
                haptic_static,
                ui_rx,
                ui_tx_for_input,
                ui_sleep,
                ui_activity,
            );
        })?;

    // Power management task
    let pwr_sleep = Arc::clone(&sleep_requested);
    let pwr_activity = Arc::clone(&last_activity_ms);
    thread::Builder::new()
        .name("power".into())
        .stack_size(STACK_POWER)
        .spawn(move || {
            tasks::power::power_task(ui_tx, pwr_sleep, pwr_activity);
        })?;

    // Main thread has nothing left to do — park it forever.
    // (All work happens in the spawned FreeRTOS tasks.)
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

// ---------------------------------------------------------------------------
// Boot helpers
// ---------------------------------------------------------------------------

/// Wait for the user to hold the button for [`BOOT_HOLD_MS`].
/// Returns `true` if the hold was completed, `false` if the button was
/// released early or a 10-second timeout elapsed.
fn wait_for_boot_hold(button: &PinDriver<'_, AnyInputPin, Input>) -> bool {
    let start = std::time::Instant::now();
    let mut held_ms: u64 = 0;
    let poll = Duration::from_millis(10);
    let timeout = Duration::from_secs(10);

    loop {
        if start.elapsed() > timeout {
            return false;
        }

        if button.is_low() {
            // Button is pressed (active LOW with pull-up).
            held_ms += 10;
            if held_ms >= BOOT_HOLD_MS {
                return true;
            }
        } else {
            held_ms = 0;
        }

        thread::sleep(poll);
    }
}

/// Configure internal pull-up on a PinDriver.  Separated because the borrow
/// checker needs a helper for the downgraded pin type.
fn configure_pullup(_pin: &PinDriver<'_, AnyInputPin, Input>) {
    // esp-idf-hal's PinDriver::input already sets the direction; we just need
    // the pull-up.  On ESP32-C3, internal pull-ups are enabled via the GPIO
    // matrix.  The PinDriver constructor with `Pull::Up` variant handles this,
    // but since we already created the driver, we set it via the raw API.
    unsafe {
        esp_idf_sys::gpio_set_pull_mode(
            PIN_BUTTON,
            esp_idf_sys::gpio_pull_mode_t_GPIO_PULLUP_ONLY,
        );
    }
}

/// Enter deep sleep with button-press wakeup.  Does not return.
fn enter_deep_sleep() -> ! {
    unsafe {
        esp_idf_sys::esp_deep_sleep_enable_gpio_wakeup(
            1u64 << PIN_BUTTON,
            esp_idf_sys::esp_deepsleep_gpio_wake_up_mode_t_ESP_GPIO_WAKEUP_GPIO_LOW,
        );
        esp_idf_sys::esp_deep_sleep_start();
    }
}
