/* Edge Impulse C Wrapper for Rust FFI
 * 
 * This file provides C-compatible wrappers around the C++ Edge Impulse SDK
 * to enable safe FFI calls from Rust.
 * 
 * Note: This wrapper requires EIDSP_SIGNAL_C_FN_POINTER=1 to be defined
 * during compilation to use C function pointers instead of std::function.
 */

#include <cstddef>
#include <cstring>
#include "edge-impulse-sdk/classifier/ei_run_classifier.h"
#include "edge-impulse-sdk/dsp/numpy.hpp"
#include "model-parameters/model_metadata.h"

extern "C" {

// Thread-local storage for the current signal being processed
// This allows the C callback to access the data pointer
thread_local static const float* g_current_signal_data = nullptr;
thread_local static size_t g_current_signal_size = 0;

// C-compatible callback function for signal_t
static int signal_get_data_callback(size_t offset, size_t length, float* out_ptr) {
    if (g_current_signal_data == nullptr) {
        return -1;
    }
    
    if (offset + length > g_current_signal_size) {
        return -1;
    }
    
    std::memcpy(out_ptr, g_current_signal_data + offset, length * sizeof(float));
    return 0;
}

/**
 * Run Edge Impulse classifier on a float buffer
 * 
 * @param features Pointer to float array of size EI_CLASSIFIER_DSP_INPUT_FRAME_SIZE
 * @param result Output structure for classification results
 * @param debug Enable debug output (0 = false, 1 = true)
 * @return 0 on success, non-zero on error
 */
int ei_run_classifier_ffi(
    const float* features,
    ei_impulse_result_t* result,
    int debug
) {
    if (features == nullptr || result == nullptr) {
        return -1;
    }
    
    // Set thread-local signal data
    g_current_signal_data = features;
    g_current_signal_size = EI_CLASSIFIER_DSP_INPUT_FRAME_SIZE;
    
    // Create signal_t structure with C function pointer
    ei::signal_t signal;
    signal.total_length = EI_CLASSIFIER_DSP_INPUT_FRAME_SIZE;
    signal.get_data = signal_get_data_callback;
    
    // Run classifier
    EI_IMPULSE_ERROR err = run_classifier(&signal, result, debug != 0);
    
    // Clear thread-local
    g_current_signal_data = nullptr;
    g_current_signal_size = 0;
    
    return (err == EI_IMPULSE_OK) ? 0 : -1;
}

/**
 * Initialize Edge Impulse classifier (for continuous inference)
 * Call this once before using run_classifier_continuous
 */
void ei_run_classifier_init_ffi(void) {
    run_classifier_init();
}

/**
 * Extract classification values from result structure
 * This helper function ensures safe access to the classification array
 * 
 * @param result Pointer to ei_impulse_result_t
 * @param out_values Output array of size EI_CLASSIFIER_LABEL_COUNT
 * @return 0 on success, non-zero on error
 */
int ei_get_classification_values(
    const ei_impulse_result_t* result,
    float* out_values
) {
    if (result == nullptr || out_values == nullptr) {
        return -1;
    }
    
    for (size_t i = 0; i < EI_CLASSIFIER_LABEL_COUNT; i++) {
        out_values[i] = result->classification[i].value;
    }
    
    return 0;
}

} // extern "C"
