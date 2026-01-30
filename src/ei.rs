// PlastiWatch V2 — Edge Impulse Inference Interface
//
// This module provides a safe Rust API for activity classification.
//
// Architecture:
//   1. STUB mode (default) — returns `Idle` so the rest of the firmware can be
//      developed and tested without the C++ Edge Impulse SDK compiled in.
//   2. FFI mode — uncomment the `edge-impulse` feature in Cargo.toml and
//      enable the build.rs EI compilation to link the real classifier.
//
// The AI task calls `classify(features)` with a 375-float buffer
// (125 samples × 3 axes) and receives back the winning label index and its
// confidence.

use crate::config::*;
use crate::events::ActivityClass;

// ---------------------------------------------------------------------------
// Public interface
// ---------------------------------------------------------------------------

/// Result of a single inference pass.
#[derive(Debug, Clone, Copy)]
pub struct ClassifierResult {
    pub activity: ActivityClass,
    pub confidence: f32,
}

/// Labels matching the Edge Impulse model output order.
pub const LABELS: [&str; EI_LABEL_COUNT] = ["idle", "snake", "updown", "wave"];

/// Run activity classification on a filled feature buffer.
///
/// `features` must contain exactly `EI_DSP_INPUT_FRAME_SIZE` (375) floats
/// representing 125 consecutive 3-axis accelerometer readings.
///
/// Returns `Some(result)` when inference succeeds and confidence exceeds the
/// threshold, or `None` when the best prediction is below threshold or an
/// error occurred.
pub fn classify(features: &[f32; EI_DSP_INPUT_FRAME_SIZE]) -> Option<ClassifierResult> {
    let predictions = run_inference(features)?;

    // Find the label with highest confidence
    let (best_idx, &best_val) = predictions
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())?;

    if best_val >= EI_CONFIDENCE_THRESHOLD {
        Some(ClassifierResult {
            activity: ActivityClass::from_label(LABELS[best_idx]),
            confidence: best_val,
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Inference back-end (swap between stub / real FFI)
// ---------------------------------------------------------------------------

/// Returns per-class confidence scores [idle, snake, updown, wave].
fn run_inference(features: &[f32; EI_DSP_INPUT_FRAME_SIZE]) -> Option<[f32; EI_LABEL_COUNT]> {
    #[cfg(not(feature = "edge-impulse"))]
    {
        return stub_inference(features);
    }

    #[cfg(feature = "edge-impulse")]
    {
        return ffi_inference(features);
    }
}

// ---------------------------------------------------------------------------
// Stub back-end — development / testing without the C++ SDK
// ---------------------------------------------------------------------------
#[cfg(not(feature = "edge-impulse"))]
fn stub_inference(_features: &[f32; EI_DSP_INPUT_FRAME_SIZE]) -> Option<[f32; EI_LABEL_COUNT]> {
    // Simple heuristic: use mean absolute acceleration to guess activity.
    // This lets the UI pipeline work end-to-end before the real model is linked.
    let mean_abs: f32 = _features.iter().map(|v| v.abs()).sum::<f32>() / _features.len() as f32;

    let preds = if mean_abs < 0.3 {
        [0.90, 0.03, 0.04, 0.03] // idle
    } else if mean_abs < 0.8 {
        [0.05, 0.05, 0.85, 0.05] // updown (walking)
    } else if mean_abs < 1.5 {
        [0.03, 0.04, 0.05, 0.88] // wave (running)
    } else {
        [0.02, 0.92, 0.03, 0.03] // snake (fall)
    };

    log::debug!(
        "STUB inference — mean |a| = {:.2}, preds = {:?}",
        mean_abs,
        preds
    );
    Some(preds)
}

// ---------------------------------------------------------------------------
// Real FFI back-end — calls the C++ Edge Impulse compiled library
// ---------------------------------------------------------------------------
#[cfg(feature = "edge-impulse")]
mod ffi {
    use std::ffi::c_char;

    #[repr(C)]
    pub struct EiSignal {
        pub get_data: Option<unsafe extern "C" fn(usize, usize, *mut f32) -> i32>,
        pub total_length: usize,
    }

    #[repr(C)]
    pub struct EiClassification {
        pub label: *const c_char,
        pub value: f32,
    }

    // The full struct has more fields; we only access `classification`.
    #[repr(C)]
    pub struct EiImpulseResult {
        pub classification: [EiClassification; super::EI_LABEL_COUNT],
        pub anomaly: f32,
    }

    extern "C" {
        pub fn run_classifier(
            signal: *mut EiSignal,
            result: *mut EiImpulseResult,
            debug: bool,
        ) -> i32;
    }
}

#[cfg(feature = "edge-impulse")]
fn ffi_inference(features: &[f32; EI_DSP_INPUT_FRAME_SIZE]) -> Option<[f32; EI_LABEL_COUNT]> {
    use std::ffi::CStr;

    // Signal callback reads directly from the features slice.
    // SAFETY: single-threaded access — only the AI task calls this.
    static mut SIGNAL_BUF: *const f32 = std::ptr::null();
    static mut SIGNAL_LEN: usize = 0;

    unsafe extern "C" fn get_data(offset: usize, length: usize, out: *mut f32) -> i32 {
        unsafe {
            if SIGNAL_BUF.is_null() || offset + length > SIGNAL_LEN {
                return -1;
            }
            core::ptr::copy_nonoverlapping(SIGNAL_BUF.add(offset), out, length);
        }
        0
    }

    unsafe {
        SIGNAL_BUF = features.as_ptr();
        SIGNAL_LEN = features.len();

        let mut signal = ffi::EiSignal {
            get_data: Some(get_data),
            total_length: features.len(),
        };

        let mut result: ffi::EiImpulseResult = core::mem::zeroed();

        let err = ffi::run_classifier(&mut signal, &mut result, false);
        if err != 0 {
            log::error!("Edge Impulse classifier error: {}", err);
            return None;
        }

        let mut preds = [0.0f32; EI_LABEL_COUNT];
        for i in 0..EI_LABEL_COUNT {
            preds[i] = result.classification[i].value;
            let label = CStr::from_ptr(result.classification[i].label);
            log::debug!("{}: {:.4}", label.to_str().unwrap_or("?"), preds[i]);
        }

        SIGNAL_BUF = std::ptr::null();
        Some(preds)
    }
}
