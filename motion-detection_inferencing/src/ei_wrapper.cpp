/*
 * Wrapper for Edge Impulse SDK to expose run_classifier as a C symbol
 * for Rust FFI.
 */

#include "edge-impulse-sdk/classifier/ei_run_classifier.h"
#include "model-parameters/model_variables.h"

extern "C" int run_classifier(ei::signal_t *signal, ei_impulse_result_t *result, bool debug) {
    return (int)process_impulse(&ei_default_impulse, signal, result, debug);
}
