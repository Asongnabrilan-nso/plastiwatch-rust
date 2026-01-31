/* Edge Impulse Porting Layer for ESP-IDF (ESP32-C3)
 * 
 * This file implements the required porting functions for Edge Impulse
 * to work with ESP-IDF on ESP32-C3.
 */

#include "edge-impulse-sdk/porting/ei_classifier_porting.h"
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <cstdarg>

// Use ESP-IDF functions
#include "esp_timer.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

// Define porting mode if not already set
#ifndef EI_PORTING_ESPRESSIF
#define EI_PORTING_ESPRESSIF 1
#endif

extern "C" {

// Timer functions
uint64_t ei_read_timer_ms() {
    return (uint64_t)(esp_timer_get_time() / 1000);
}

uint64_t ei_read_timer_us() {
    return (uint64_t)esp_timer_get_time();
}

// Sleep function
EI_IMPULSE_ERROR ei_sleep(int32_t time_ms) {
    if (time_ms < 0) {
        return EI_IMPULSE_OK;
    }
    vTaskDelay(pdMS_TO_TICKS(time_ms));
    return EI_IMPULSE_OK;
}

// Print functions - use ESP-IDF logging
// Note: esp_log_writev expects a format string and va_list, but we need to format first
// For simplicity, we'll use a small buffer (Edge Impulse debug messages are typically short)
void ei_printf(const char *format, ...) {
    char buffer[256];
    va_list args;
    va_start(args, format);
    int len = vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);
    if (len > 0 && len < (int)sizeof(buffer)) {
        ESP_LOGI("EI", "%s", buffer);
    }
}

void ei_printf_float(float f) {
    ei_printf("%.6f", f);
}

// Character I/O - not typically used in embedded, but provide stubs
void ei_putchar(char c) {
    // Can be implemented to send over UART if needed
    // For now, just use printf
    ei_printf("%c", c);
}

char ei_getchar(void) {
    // Not typically used in embedded inference
    return 0;
}

// Memory management - use FreeRTOS heap
void *ei_malloc(size_t size) {
    return pvPortMalloc(size);
}

void *ei_calloc(size_t nitems, size_t size) {
    void *ptr = pvPortMalloc(nitems * size);
    if (ptr != nullptr) {
        memset(ptr, 0, nitems * size);
    }
    return ptr;
}

void ei_free(void *ptr) {
    vPortFree(ptr);
}

// Serial baudrate - not used in our case
void ei_serial_set_baudrate(int baudrate) {
    // Not implemented - not needed for inference-only use
    (void)baudrate;
}

// Cancel check - not used in our case
EI_IMPULSE_ERROR ei_run_impulse_check_canceled() {
    return EI_IMPULSE_OK;
}

} // extern "C"
