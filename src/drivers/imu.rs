// PlastiWatch V2 — MPU6050 IMU Driver
//
// Custom register-level driver over shared I2C bus.
// Avoids external crate version conflicts with esp-idf-hal.

use std::sync::Mutex;

use esp_idf_hal::i2c::I2cDriver;

use crate::config::*;
use crate::events::SensorData;

/// Thread-safe handle to a shared I2C bus.
pub type SharedBus = &'static Mutex<I2cDriver<'static>>;

// MPU6050 register addresses
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_CONFIG: u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG: u8 = 0x1C;
const REG_ACCEL_XOUT_H: u8 = 0x3B; // Start of 14-byte sensor burst
const REG_WHO_AM_I: u8 = 0x75;
const WHO_AM_I_EXPECTED: u8 = 0x68;

pub struct Mpu6050 {
    bus: SharedBus,
}

impl Mpu6050 {
    pub fn new(bus: SharedBus) -> Self {
        Self { bus }
    }

    /// Verify the device is reachable on the I2C bus.
    pub fn is_connected(&self) -> bool {
        let mut bus = self.bus.lock().unwrap();
        let mut buf = [0u8; 1];
        match bus.write_read(I2C_ADDR_MPU6050, &[REG_WHO_AM_I], &mut buf, I2C_TIMEOUT_TICKS) {
            Ok(()) => buf[0] == WHO_AM_I_EXPECTED,
            Err(_) => false,
        }
    }

    /// Wake the sensor and configure accel (±8 g), gyro (±500 °/s), DLPF 21 Hz.
    pub fn init(&self) -> anyhow::Result<()> {
        let mut bus = self.bus.lock().unwrap();

        // Wake up (clear SLEEP bit)
        bus.write(I2C_ADDR_MPU6050, &[REG_PWR_MGMT_1, 0x00], I2C_TIMEOUT_TICKS)?;

        // DLPF bandwidth 21 Hz
        bus.write(I2C_ADDR_MPU6050, &[REG_CONFIG, 0x04], I2C_TIMEOUT_TICKS)?;

        // Gyroscope: ±500 °/s
        bus.write(I2C_ADDR_MPU6050, &[REG_GYRO_CONFIG, 0x08], I2C_TIMEOUT_TICKS)?;

        // Accelerometer: ±8 g
        bus.write(I2C_ADDR_MPU6050, &[REG_ACCEL_CONFIG, 0x10], I2C_TIMEOUT_TICKS)?;

        log::info!("MPU6050 initialised (±8g, ±500°/s, DLPF 21Hz)");
        Ok(())
    }

    /// Burst-read all 6 axes and convert to physical units.
    pub fn read_data(&self) -> anyhow::Result<SensorData> {
        let mut bus = self.bus.lock().unwrap();
        let mut raw = [0u8; 14];
        bus.write_read(
            I2C_ADDR_MPU6050,
            &[REG_ACCEL_XOUT_H],
            &mut raw,
            I2C_TIMEOUT_TICKS,
        )?;

        Ok(SensorData {
            ax: i16::from_be_bytes([raw[0], raw[1]]) as f32 / ACCEL_SCALE_8G,
            ay: i16::from_be_bytes([raw[2], raw[3]]) as f32 / ACCEL_SCALE_8G,
            az: i16::from_be_bytes([raw[4], raw[5]]) as f32 / ACCEL_SCALE_8G,
            // raw[6..8] = temperature — skipped
            gx: i16::from_be_bytes([raw[8], raw[9]]) as f32 / GYRO_SCALE_500,
            gy: i16::from_be_bytes([raw[10], raw[11]]) as f32 / GYRO_SCALE_500,
            gz: i16::from_be_bytes([raw[12], raw[13]]) as f32 / GYRO_SCALE_500,
        })
    }
}
