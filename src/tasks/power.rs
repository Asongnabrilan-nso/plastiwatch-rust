// PlastiWatch V2 — Power Management Task
//
// Periodically reads battery voltage, sends updates to the UI, and handles
// deep-sleep entry on long-press or inactivity timeout.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config::*;
use crate::events::UiEvent;

pub fn power_task(
    ui_tx: Sender<UiEvent>,
    sleep_requested: Arc<AtomicBool>,
    last_activity_ms: Arc<AtomicU32>,
) {
    log::info!("Power task started");

    let check_interval = Duration::from_millis(BATTERY_CHECK_INTERVAL_MS);

    // One-time ADC setup via raw ESP-IDF calls.
    // GPIO2 / ADC1_CHANNEL_2 with 11 dB attenuation (0–3.3 V range).
    unsafe {
        let mut handle: esp_idf_sys::adc_oneshot_unit_handle_t = core::ptr::null_mut();
        let unit_cfg = esp_idf_sys::adc_oneshot_unit_init_cfg_t {
            unit_id: esp_idf_sys::adc_unit_t_ADC_UNIT_1,
            ulp_mode: esp_idf_sys::adc_ulp_mode_t_ADC_ULP_MODE_DISABLE,
            ..core::mem::zeroed()
        };
        let ret = esp_idf_sys::adc_oneshot_new_unit(&unit_cfg, &mut handle);
        if ret != esp_idf_sys::ESP_OK {
            log::error!("ADC unit init failed ({})", ret);
        }

        let chan_cfg = esp_idf_sys::adc_oneshot_chan_cfg_t {
            atten: esp_idf_sys::adc_atten_t_ADC_ATTEN_DB_11,
            bitwidth: esp_idf_sys::adc_bitwidth_t_ADC_BITWIDTH_12,
        };
        let channel = esp_idf_sys::adc_channel_t_ADC_CHANNEL_2; // GPIO2
        let ret = esp_idf_sys::adc_oneshot_config_channel(handle, channel, &chan_cfg);
        if ret != esp_idf_sys::ESP_OK {
            log::error!("ADC channel config failed ({})", ret);
        }

        loop {
            // ---- Check for sleep request (long-press) ----
            if sleep_requested.load(Ordering::SeqCst) {
                enter_deep_sleep();
            }

            // ---- Check inactivity timeout ----
            let last = last_activity_ms.load(Ordering::Relaxed);
            let now = crate::now_ms();
            if now.wrapping_sub(last) > INACTIVITY_TIMEOUT_MS {
                log::info!("Inactivity timeout ({} ms) — entering deep sleep", INACTIVITY_TIMEOUT_MS);
                enter_deep_sleep();
            }

            // ---- Read battery voltage ----
            let mut raw: i32 = 0;
            let ret = esp_idf_sys::adc_oneshot_read(handle, channel, &mut raw);
            if ret == esp_idf_sys::ESP_OK {
                // Assumes a 1:2 resistor divider before the ADC pin.
                let voltage = (raw as f32 / 4095.0) * 3.3 * 2.0;
                // Map LiPo range: 3.3 V = 0%, 4.2 V = 100%
                let level = ((voltage - 3.3) / (4.2 - 3.3) * 100.0).clamp(0.0, 100.0);

                let _ = ui_tx.send(UiEvent::UpdateBattery(level));
            }

            thread::sleep(check_interval);
        }
    }
}

/// Configure GPIO wakeup on button press and enter deep sleep.
/// This function does not return.
fn enter_deep_sleep() -> ! {
    log::info!("Entering deep sleep — wake on button press (GPIO{})", PIN_BUTTON);
    unsafe {
        esp_idf_sys::esp_deep_sleep_enable_gpio_wakeup(
            1u64 << PIN_BUTTON,
            esp_idf_sys::esp_deepsleep_gpio_wake_up_mode_t_ESP_GPIO_WAKEUP_GPIO_LOW,
        );
        esp_idf_sys::esp_deep_sleep_start();
    }
    // Never reached — but satisfies the `!` return type.

}
