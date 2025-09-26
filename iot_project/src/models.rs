use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---------- API de teste simples ----------
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateItem {
    pub nome: String,
}

// ---------- Estruturas do JSON publicado pelo ESP32 ----------
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SensorPayload {
    pub ts: i64,                    // timestamp (epoch)
    pub device: String,             // ID do dispositivo
    pub wifi: Option<Wifi>,         // Info WiFi
    pub bno055: Option<Bno055>,     // Dados do sensor BNO055
    pub ultrasonic_cm: Option<f32>, // Distância do ultrassônico
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Wifi {
    pub rssi: i32, // intensidade do sinal
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Bno055 {
    pub ok: bool,                        // se o sensor está OK
    pub heading_deg: Option<f32>,        // yaw
    pub roll_deg: Option<f32>,           // roll
    pub pitch_deg: Option<f32>,          // pitch
    pub temp_c: Option<f32>,             // temperatura interna
    pub linear_accel_ms2: Option<Vector3>, // aceleração linear
    pub gyro_rads: Option<Vector3>,        // giroscópio
    pub mag_uT: Option<Vector3>,           // magnetômetro
    pub calib: Option<Calib>,              // calibração
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Calib {
    pub sys: i32,
    pub gyro: i32,
    pub accel: i32,
    pub mag: i32,
}

// ---------- Para mapear linhas do banco (sensor_data) ----------
#[derive(Debug, FromRow, Serialize, Clone)]
pub struct SensorDataRow {
    pub id: i32,
    pub ts: chrono::NaiveDateTime,
    pub device: String,
    pub wifi_rssi: Option<i32>,
    pub bno_ok: Option<bool>,
    pub heading_deg: Option<f32>,
    pub roll_deg: Option<f32>,
    pub pitch_deg: Option<f32>,
    pub temp_c: Option<f32>,
    pub accel_x: Option<f32>,
    pub accel_y: Option<f32>,
    pub accel_z: Option<f32>,
    pub gyro_x: Option<f32>,
    pub gyro_y: Option<f32>,
    pub gyro_z: Option<f32>,
    pub mag_x: Option<f32>,
    pub mag_y: Option<f32>,
    pub mag_z: Option<f32>,
    pub calib_sys: Option<i32>,
    pub calib_gyro: Option<i32>,
    pub calib_accel: Option<i32>,
    pub calib_mag: Option<i32>,
    pub ultrasonic_cm: Option<f32>,
}
