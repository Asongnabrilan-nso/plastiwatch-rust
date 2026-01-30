// PlastiWatch V2 — AI Inference Task
//
// Buffers 125 accelerometer samples (2-second window at 62.5 Hz), then runs
// the Edge Impulse classifier.  When confidence exceeds the threshold, the
// detected activity is forwarded to the UI task.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use crate::config::*;
use crate::ei;
use crate::events::{SensorData, UiEvent};

pub fn ai_task(
    sensor_rx: Receiver<SensorData>,
    ui_tx: Sender<UiEvent>,
    last_activity_ms: Arc<AtomicU32>,
) {
    log::info!("AI task started");

    let mut features = [0.0f32; EI_DSP_INPUT_FRAME_SIZE];
    let mut feature_ix: usize = 0;

    loop {
        // Block until a sensor sample arrives.
        let data = match sensor_rx.recv() {
            Ok(d) => d,
            Err(_) => {
                log::warn!("Sensor channel closed — exiting AI task");
                return;
            }
        };

        // Accumulate 3-axis accelerometer values into the feature buffer.
        if feature_ix + EI_RAW_SAMPLES_PER_FRAME > EI_DSP_INPUT_FRAME_SIZE {
            // Safety guard — should never happen, but reset gracefully.
            feature_ix = 0;
        }

        features[feature_ix] = data.ax;
        features[feature_ix + 1] = data.ay;
        features[feature_ix + 2] = data.az;
        feature_ix += EI_RAW_SAMPLES_PER_FRAME;

        // Once the buffer is full (125 samples), run inference.
        if feature_ix >= EI_DSP_INPUT_FRAME_SIZE {
            if let Some(result) = ei::classify(&features) {
                log::info!(
                    "Activity: {:?} ({:.1}%)",
                    result.activity,
                    result.confidence * 100.0
                );

                // Update the activity timestamp (prevents inactivity sleep while moving).
                last_activity_ms.store(crate::now_ms(), Ordering::Relaxed);

                let _ = ui_tx.send(UiEvent::UpdateActivity(result.activity));
            }

            // Reset buffer for the next window.
            feature_ix = 0;
        }
    }
}
