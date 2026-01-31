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
    #[cfg(feature = "edge-impulse")]
    init_classifier();
    
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
// Real FFI back-end — calls the C++ Edge Impulse compiled library via C wrapper
// ---------------------------------------------------------------------------
#[cfg(feature = "edge-impulse")]
mod ffi {
    use std::ffi::c_char;

    // Match ei_impulse_result_classification_t from ei_classifier_types.h
    #[repr(C)]
    pub struct EiClassification {
        pub label: *const c_char,
        pub value: f32,
    }

    // Match ei_impulse_result_timing_t (simplified - we only need what's used)
    #[repr(C)]
    pub struct EiTiming {
        pub sampling: i32,
        pub dsp: i32,
        pub classification: i32,
        pub anomaly: i32,
        pub dsp_us: i64,
        pub classification_us: i64,
        pub anomaly_us: i64,
    }

    // Match ei_impulse_result_t from ei_classifier_types.h
    // Note: This matches the statically allocated version (EI_IMPULSE_RESULT_CLASSIFICATION_IS_STATICALLY_ALLOCATED == 1)
    // Structure layout matches C++ version compiled with our model settings:
    // - EI_CLASSIFIER_OBJECT_DETECTION = 0 (no bounding boxes)
    // - EI_CLASSIFIER_HAS_VISUAL_ANOMALY = 0 (no visual AD fields)
    // - EI_CLASSIFIER_HR_ENABLED = 0 (no HR fields)
    // Note: On ESP32-C3 (32-bit RISC-V), pointers are 4 bytes
    #[repr(C)]
    pub struct EiImpulseResult {
        pub bounding_boxes: *mut core::ffi::c_void, // Pointer (4 bytes on 32-bit)
        pub bounding_boxes_count: u32,              // 4 bytes
        pub classification: [EiClassification; super::EI_LABEL_COUNT], // Array of 4 structs
        pub anomaly: f32,                           // 4 bytes
        pub timing: EiTiming,                       // Struct with i32, i32, i32, i32, i64, i64, i64
        pub _raw_outputs: *mut core::ffi::c_void,   // C++ pointer (4 bytes on 32-bit)
        // Visual AD fields not present (EI_CLASSIFIER_HAS_VISUAL_ANOMALY == 0)
        // HR fields not present (EI_CLASSIFIER_HR_ENABLED == 0)
        pub postprocessed_output: [u8; 0],          // Empty struct (0 bytes)
    }

    // C wrapper functions from ei_wrapper.cpp
    extern "C" {
        /// Run Edge Impulse classifier on a float buffer
        /// Returns 0 on success, non-zero on error
        pub fn ei_run_classifier_ffi(
            features: *const f32,
            result: *mut EiImpulseResult,
            debug: i32,
        ) -> i32;

        /// Initialize Edge Impulse classifier (for continuous inference)
        pub fn ei_run_classifier_init_ffi();

        /// Extract classification values from result structure
        /// Returns 0 on success, non-zero on error
        pub fn ei_get_classification_values(
            result: *const EiImpulseResult,
            out_values: *mut f32,
        ) -> i32;
    }
}

#[cfg(feature = "edge-impulse")]
fn ffi_inference(features: &[f32; EI_DSP_INPUT_FRAME_SIZE]) -> Option<[f32; EI_LABEL_COUNT]> {
    use std::ffi::CStr;

    unsafe {
        // Zero-initialize the result structure
        let mut result: ffi::EiImpulseResult = core::mem::zeroed();

        // Call the C wrapper function
        let err = ffi::ei_run_classifier_ffi(
            features.as_ptr(),
            &mut result,
            0, // debug = false
        );

        if err != 0 {
            log::error!("Edge Impulse classifier error: {}", err);
            return None;
        }

        // Extract classification results using the safe helper function
        let mut preds = [0.0f32; EI_LABEL_COUNT];
        let extract_err = ffi::ei_get_classification_values(&result, preds.as_mut_ptr());
        if extract_err != 0 {
            log::error!("Failed to extract classification values");
            return None;
        }
        
        // Log the results (try to get labels from the result structure)
        for i in 0..EI_LABEL_COUNT {
            if !result.classification[i].label.is_null() {
                if let Ok(label_cstr) = CStr::from_ptr(result.classification[i].label) {
                    if let Ok(label_str) = label_cstr.to_str() {
                        log::debug!("{}: {:.4}", label_str, preds[i]);
                    }
                }
            }
        }

        // Log timing information
        log::debug!(
            "Inference timing — DSP: {} ms, Classification: {} ms, Anomaly: {} ms",
            result.timing.dsp,
            result.timing.classification,
            result.timing.anomaly
        );

        Some(preds)
    }
}

// Initialize Edge Impulse on first use (called from main or AI task)
#[cfg(feature = "edge-impulse")]
static EI_INIT_ONCE: std::sync::Once = std::sync::Once::new();

#[cfg(feature = "edge-impulse")]
pub fn init_classifier() {
    EI_INIT_ONCE.call_once(|| {
        unsafe {
            ffi::ei_run_classifier_init_ffi();
        }
        log::info!("Edge Impulse classifier initialized");
    });
}
