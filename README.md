# 📡 IoT Project – ESP32 + Rust + PostgreSQL + MQTT + 3D Viewer

#GABRIEL, EDUARDO
Este projeto integit gra sensores de um **ESP32** (BNO055 + Ultrassônico) via **MQTT**, armazena os dados em um banco **PostgreSQL** usando uma API em **Rust (Axum + SQLx)** e exibe em tempo real em uma página web com **Three.js** (cubo 3D que gira e escala conforme os sensores).

---

## 🔧 Tecnologias

- **Rust + Axum** → API web e servidor WebSocket
- **SQLx + PostgreSQL** → persistência dos dados dos sensores
- **paho-mqtt** → cliente MQTT assíncrono em Rust
- **Three.js** → renderização do cubo 3D no navegador
- **ESP32 + PubSubClient** → publica JSON no broker MQTT

---

## 📦 Estrutura

```
iot_project/
 ├─ src/
 │   ├─ main.rs      # servidor principal
 │   ├─ db.rs        # inicialização banco
 │   ├─ models.rs    # structs (Item + SensorPayload)
 ├─ Cargo.toml
 ├─ .env
 └─ README.md
```

---

## 🗄️ Banco de Dados

Crie a tabela no PostgreSQL:

```sql
CREATE TABLE IF NOT EXISTS items (
    id UUID PRIMARY KEY,
    nome TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sensor_data (
    id SERIAL PRIMARY KEY,
    ts TIMESTAMP NOT NULL,
    device TEXT NOT NULL,
    wifi_rssi INT,
    bno_ok BOOL,
    heading_deg FLOAT,
    roll_deg FLOAT,
    pitch_deg FLOAT,
    temp_c FLOAT,
    accel_x FLOAT,
    accel_y FLOAT,
    accel_z FLOAT,
    gyro_x FLOAT,
    gyro_y FLOAT,
    gyro_z FLOAT,
    mag_x FLOAT,
    mag_y FLOAT,
    mag_z FLOAT,
    calib_sys INT,
    calib_gyro INT,
    calib_accel INT,
    calib_mag INT,
    ultrasonic_cm FLOAT
);
```

---

## ⚙️ Configuração

Crie o arquivo **`.env`**:

```env
DATABASE_URL=postgres://usuario:senha@localhost:5432/iot_db
```

---

## ▶️ Rodando

```bash
# 1. Instalar dependências do sistema
sudo apt update
sudo apt install -y cmake libssl-dev build-essential pkg-config

# 2. Build do projeto
cargo build

# 3. Executar
cargo run
```

Ao rodar, você verá:

```
✅ Conectado ao banco PostgreSQL em: postgres://...
📡 Inscrito no tópico devices/esp32/+/state
🚀 Servidor rodando em http://127.0.0.1:43625
```

---

## 🌐 Endpoints

- `GET /` → página HTML com cubo 3D
- `GET /ws` → WebSocket com streaming de dados
- `GET /items` → lista de itens cadastrados
- `POST /items` → cria item (JSON `{ "nome": "teste" }`)
- `GET /data` → últimos 50 registros
- `GET /data/:device` → últimos 50 registros de um device específico

---

## 🎮 Página 3D

A página `/` abre um cubo 3D:
- Rotação baseada em **Yaw/Pitch/Roll** do BNO055
- Escala baseada no **ultrassônico**
- HUD mostra dados ao vivo (`device`, `yaw`, `pitch`, `roll`, `distância`)

---

## 📡 ESP32 → MQTT JSON

O ESP32 publica em `devices/esp32/<deviceId>/state` um JSON como:

```json
{
  "ts": 1758928169,
  "device": "4C1C1CBF713C",
  "wifi": { "rssi": -53 },
  "bno055": {
    "ok": true,
    "heading_deg": 348.8,
    "roll_deg": 1.3,
    "pitch_deg": -47.4,
    "temp_c": 26,
    "linear_accel_ms2": { "x": -0.05, "y": -0.05, "z": -0.03 },
    "gyro_rads": { "x": 0, "y": 0, "z": -0.002 },
    "mag_uT": { "x": -16.2, "y": -11.6, "z": 18 },
    "calib": { "sys": 0, "gyro": 3, "accel": 1, "mag": 1 }
  },
  "ultrasonic_cm": 27.7
}
```

Este payload é automaticamente:
1. Armazenado no **Postgres**
2. Enviado via **WebSocket** para os navegadores conectados

---

## ✅ Conclusão

Com isso você tem:
- Captura de dados do ESP32 via MQTT
- Armazenamento confiável em Postgres
- API e rotas REST em Rust
- Streaming em tempo real para front-end 3D