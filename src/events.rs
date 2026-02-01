// PlastiWatch V2 — System Events & Data Types

// ---------------------------------------------------------------------------
// Sensor Data (6-axis IMU reading from MPU6050)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, Default)]
pub struct SensorData {
    pub ax: f32,
    pub ay: f32,
    pub az: f32,
    pub gx: f32,
    pub gy: f32,
    pub gz: f32,
}

// ---------------------------------------------------------------------------
// Activity Classification
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityClass {
    Idle,
    Snake,
    UpDown,
    Wave,
}

impl ActivityClass {
    /// Human-readable label (kept for debugging/logging purposes).
    #[allow(dead_code)]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Idle   => "normal",
            Self::Snake  => "fall!",
            Self::UpDown => "walking",
            Self::Wave   => "running",
        }
    }

    /// Map an Edge Impulse label string to an `ActivityClass`.
    pub fn from_label(label: &str) -> Self {
        match label {
            "idle"   => Self::Idle,
            "snake"  => Self::Snake,
            "updown" => Self::UpDown,
            "wave"   => Self::Wave,
            _        => Self::Idle,
        }
    }
}

impl Default for ActivityClass {
    fn default() -> Self {
        Self::Idle
    }
}

// ---------------------------------------------------------------------------
// UI Events — sent to the UI task via channel
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    /// AI classified a new activity.
    UpdateActivity(ActivityClass),
    /// Battery level changed (0.0–100.0 %).
    UpdateBattery(f32),
    /// Single button click detected.
    ButtonSingleClick,
    /// Double button click detected.
    ButtonDoubleClick,
    /// Long button press (≥ 3 s) detected.
    ButtonLongPress,
}
