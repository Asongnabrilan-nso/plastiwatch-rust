/* Edge Impulse C Wrapper Header for Rust FFI */

#ifndef EI_WRAPPER_H
#define EI_WRAPPER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Forward declaration - actual definition is in ei_classifier_types.h
typedef struct ei_impulse_result_t ei_impulse_result_t;

/**
 * Run Edge Impulse classifier on a float buffer
 * 
 * @param features Pointer to float array of size EI_CLASSIFIER_DSP_INPUT_FRAME_SIZE (375)
 * @param result Output structure for classification results
 * @param debug Enable debug output (0 = false, 1 = true)
 * @return 0 on success, non-zero on error
 */
int ei_run_classifier_ffi(
    const float* features,
    ei_impulse_result_t* result,
    int debug
);

/**
 * Initialize Edge Impulse classifier (for continuous inference)
 * Call this once before using run_classifier_continuous
 */
void ei_run_classifier_init_ffi(void);

/**
 * Extract classification values from result structure
 * 
 * @param result Pointer to ei_impulse_result_t
 * @param out_values Output array of size EI_CLASSIFIER_LABEL_COUNT (4)
 * @return 0 on success, non-zero on error
 */
int ei_get_classification_values(
    const ei_impulse_result_t* result,
    float* out_values
);

#ifdef __cplusplus
}
#endif

#endif // EI_WRAPPER_H
