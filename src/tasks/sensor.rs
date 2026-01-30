// PlastiWatch V2 — Sensor Task
//
// Continuously reads 6-axis IMU data at ~62.5 Hz and pushes samples into the
// sensor channel for the AI task to consume.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::*;
use crate::drivers::imu::{Mpu6050, SharedBus};
use crate::events::SensorData;

pub fn sensor_task(bus: SharedBus, sensor_tx: Sender<SensorData>) {
    log::info!("Sensor task started");

    let imu = Mpu6050::new(bus);
    if let Err(e) = imu.init() {
        log::error!("MPU6050 init failed in sensor task: {}", e);
        return;
    }

    let interval = Duration::from_millis(SENSOR_SAMPLE_INTERVAL_MS);

    loop {
        let tick_start = Instant::now();

        match imu.read_data() {
            Ok(data) => {
                // Non-blocking send: if the AI task is behind, drop the oldest
                // samples rather than blocking the sensor.
                if sensor_tx.send(data).is_err() {
                    // Receiver dropped — AI task has exited. Shut down cleanly.
                    log::warn!("Sensor channel closed — exiting sensor task");
                    return;
                }
            }
            Err(e) => {
                log::warn!("IMU read error: {}", e);
            }
        }

        // Sleep for the remainder of the sampling interval to maintain ~62.5 Hz.
        let elapsed = tick_start.elapsed();
        if elapsed < interval {
            thread::sleep(interval - elapsed);
        }
    }
}
