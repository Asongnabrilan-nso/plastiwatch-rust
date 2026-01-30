// PlastiWatch V2 — Hardware & System Configuration
// Target: Seeed Studio Xiao ESP32-C3 (RISC-V)

// ---------------------------------------------------------------------------
// GPIO Pin Definitions (Xiao ESP32-C3 pinout)
// ---------------------------------------------------------------------------
pub const PIN_BUTTON: i32 = 3;      // D1/A1 — User button (INPUT_PULLUP, active LOW)
pub const PIN_HAPTIC: i32 = 4;      // D2/A2 — Haptic motor control
pub const PIN_I2C_SDA: i32 = 6;     // D4    — I2C data line
pub const PIN_I2C_SCL: i32 = 7;     // D5    — I2C clock line
pub const PIN_BATTERY_ADC: u32 = 2; // D0/A0 — Battery voltage (ADC)

// ---------------------------------------------------------------------------
// I2C Bus
// ---------------------------------------------------------------------------
pub const I2C_ADDR_MPU6050: u8 = 0x68;
pub const I2C_ADDR_OLED: u8 = 0x3C;
pub const I2C_TIMEOUT_TICKS: u32 = 1000; // FreeRTOS ticks

// ---------------------------------------------------------------------------
// Display (SSD1306 OLED)
// ---------------------------------------------------------------------------
pub const SCREEN_WIDTH: u32 = 128;
pub const SCREEN_HEIGHT: u32 = 64;
pub const DISPLAY_BUFFER_SIZE: usize = (SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize) / 8; // 1024

// ---------------------------------------------------------------------------
// Task Stack Sizes (bytes)
// ---------------------------------------------------------------------------
pub const STACK_SENSOR: usize = 4096;
pub const STACK_AI: usize = 8192;
pub const STACK_UI: usize = 8192;
pub const STACK_POWER: usize = 4096;

// ---------------------------------------------------------------------------
// Timing (milliseconds)
// ---------------------------------------------------------------------------
pub const SENSOR_SAMPLE_INTERVAL_MS: u64 = 16;        // ~62.5 Hz
pub const UI_POLL_INTERVAL_MS: u64 = 10;               // 100 Hz input poll / refresh
pub const BATTERY_CHECK_INTERVAL_MS: u64 = 10_000;     // 10 seconds
pub const DEBOUNCE_MS: u64 = 50;
pub const LONG_PRESS_MS: u64 = 3000;                   // 3-second hold
pub const DOUBLE_CLICK_WINDOW_MS: u64 = 400;
pub const BOOT_HOLD_MS: u64 = 3000;                    // 3-second boot trigger
pub const INACTIVITY_TIMEOUT_MS: u32 = 180_000;        // 3 minutes → sleep
pub const BOOT_LOGO_DISPLAY_MS: u64 = 1000;            // Logo splash duration
pub const BOOT_TEXT_DISPLAY_MS: u64 = 1000;             // Text splash duration

// ---------------------------------------------------------------------------
// AI / Edge Impulse Model
// ---------------------------------------------------------------------------
pub const EI_RAW_SAMPLES_PER_FRAME: usize = 3;   // accX, accY, accZ
pub const EI_RAW_SAMPLE_COUNT: usize = 125;       // 2-second window @ 62.5 Hz
pub const EI_DSP_INPUT_FRAME_SIZE: usize = EI_RAW_SAMPLE_COUNT * EI_RAW_SAMPLES_PER_FRAME; // 375
pub const EI_LABEL_COUNT: usize = 4;
pub const EI_CONFIDENCE_THRESHOLD: f32 = 0.7;

// ---------------------------------------------------------------------------
// MPU6050 Sensor Scale Factors
// ---------------------------------------------------------------------------
pub const ACCEL_SCALE_8G: f32 = 4096.0;   // LSB/g  at ±8 g
pub const GYRO_SCALE_500: f32 = 65.5;     // LSB/°/s at ±500 °/s
