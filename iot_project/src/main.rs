use axum::{
    extract::{Path, State},
    response::{Html, Json},
    routing::{get, post},
    Json as AxumJson, Router,
};
use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use sqlx::PgPool;
use uuid::Uuid;
use dotenvy::dotenv;
use std::{env, time::Duration};
use tokio::{net::TcpListener, sync::broadcast};
use paho_mqtt as mqtt;

mod db;
mod models;

use db::{Item, init_db};
use models::{CreateItem, SensorPayload};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    tx: broadcast::Sender<String>, // broadcasting JSON para os browsers
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("❌ DATABASE_URL não definido no .env");
    let pool = PgPool::connect(&db_url).await?;
    init_db(&pool).await?;
    println!("✅ Conectado ao banco PostgreSQL em: {}", db_url);

    // Canal para enviar dados em tempo real aos clientes WebSocket
    let (tx, _rx) = broadcast::channel::<String>(100);

    // Sobe o subscriber MQTT em paralelo
    let mqtt_state = AppState { pool: pool.clone(), tx: tx.clone() };
    tokio::spawn(async move {
        if let Err(e) = mqtt_subscribe(mqtt_state).await {
            eprintln!("❌ Erro MQTT: {:?}", e);
        }
    });

    // Rotas HTTP/WS
    let app_state = AppState { pool, tx };
    let app = Router::new()
        // Página com o cubo 3D
        .route("/", get(cube_page))
        // WebSocket para streaming dos dados
        .route("/ws", get(ws_handler))
        // Suas rotas existentes
        .route("/items", post(create_item).get(list_items))
        .route("/data", get(list_data))
        .route("/data/:device", get(list_data_by_device))
        .with_state(app_state);

    // Porta aleatória
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;
    println!("🚀 Servidor rodando em http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

/* ----------------------------- Página 3D ----------------------------- */

async fn cube_page() -> Html<String> {
    // Página simples com Three.js via CDN. Conecta no WS e atualiza o cubo.
    let html = r#"
<!doctype html>
<html lang="pt-br">
<head>
  <meta charset="utf-8" />
  <title>Cube 3D Live</title>
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <style>
    html, body { margin:0; height:100%; background:#0b1220; color:#eaeaea; font-family:system-ui, sans-serif; }
    #hud { position:fixed; top:8px; left:8px; background:rgba(0,0,0,.35); padding:10px 12px; border-radius:10px; font-size:14px; }
    canvas { display:block; }
  </style>
</head>
<body>
  <div id="hud">Aguardando dados...</div>
  <script src="https://unpkg.com/three@0.160.0/build/three.min.js"></script>
  <script>
    // Cena básica
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(60, window.innerWidth/window.innerHeight, 0.1, 100);
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(window.innerWidth, window.innerHeight);
    document.body.appendChild(renderer.domElement);

    // Luz
    const light = new THREE.DirectionalLight(0xffffff, 1);
    light.position.set(2, 3, 4);
    scene.add(light);
    scene.add(new THREE.AmbientLight(0x888888));

    // Cubo
    const geo = new THREE.BoxGeometry(1, 1, 1);
    const mat = new THREE.MeshStandardMaterial({ color: 0x3da9fc, metalness: 0.15, roughness: 0.35 });
    const cube = new THREE.Mesh(geo, mat);
    scene.add(cube);

    camera.position.set(0, 0, 3);

    // Ajuste viewport
    window.addEventListener('resize', () => {
      camera.aspect = window.innerWidth/window.innerHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(window.innerWidth, window.innerHeight);
    });

    // HUD
    const hud = document.getElementById('hud');

    // Util: graus -> radianos
    const toRad = d => d * Math.PI / 180;

    // Mapeia distância (cm) para escala (0.6 a 2.0)
    function scaleFromDistance(cm) {
      if (cm == null || isNaN(cm)) return 1.0;
      const minCm = 10, maxCm = 200, minS = 0.6, maxS = 2.0;
      const clamped = Math.max(minCm, Math.min(maxCm, cm));
      const t = (clamped - minCm) / (maxCm - minCm);
      return minS + t * (maxS - minS);
    }

    // Animação
    function animate() {
      requestAnimationFrame(animate);
      renderer.render(scene, camera);
    }
    animate();

    // WebSocket (usa host atual)
    const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${scheme}://${location.host}/ws`);

    ws.onopen = () => { hud.textContent = 'Conectado. Aguardando dados do sensor...'; };
    ws.onclose = () => { hud.textContent = 'WS fechado.'; };
    ws.onerror = () => { hud.textContent = 'Erro WS.'; };

    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data);

        // Yaw (heading_deg) → eixo Y
        // Pitch (pitch_deg)  → eixo X
        // Roll (roll_deg)    → eixo Z
        const yaw   = msg?.bno055?.heading_deg ?? 0;
        const pitch = msg?.bno055?.pitch_deg   ?? 0;
        const roll  = msg?.bno055?.roll_deg    ?? 0;

        cube.rotation.set(toRad(pitch), toRad(yaw), toRad(roll));

        const dist = msg?.ultrasonic_cm ?? null;
        const s = scaleFromDistance(dist);
        cube.scale.set(s, s, s);

        hud.textContent = `Device: ${msg.device || '-'} | Yaw:${yaw.toFixed(1)}° Pitch:${pitch.toFixed(1)}° Roll:${roll.toFixed(1)}° | Dist:${dist ? dist.toFixed(1) + 'cm' : '–'}`;
      } catch (e) {
        // ignora
      }
    };
  </script>
</body>
</html>
"#;
    Html(html.to_string())
}

/* ----------------------------- WebSocket ----------------------------- */

async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|socket| ws_loop(socket, state))
}

async fn ws_loop(mut socket: WebSocket, state: AppState) {
    // Cada cliente ganha um receiver
    let mut rx = state.tx.subscribe();

    // Loop: repassa as mensagens (JSON) publicadas pelo MQTT
    loop {
        tokio::select! {
            Ok(json) = rx.recv() => {
                let _ = socket.send(Message::Text(json)).await;
            }
            // Se o client fechar, esse send vai falhar e a task termina
            else => break,
        }
    }
}

/* ----------------------------- Rotas /items & /data ----------------------------- */

async fn create_item(
    State(state): State<AppState>,
    AxumJson(payload): AxumJson<CreateItem>
) -> Json<Item> {
    let item = Item { id: Uuid::new_v4().to_string(), nome: payload.nome };

    sqlx::query("INSERT INTO items (id, nome) VALUES ($1, $2)")
        .bind(&item.id).bind(&item.nome)
        .execute(&state.pool).await.unwrap();

    Json(item)
}

async fn list_items(State(state): State<AppState>) -> Json<Vec<Item>> {
    let items = sqlx::query_as::<_, Item>("SELECT id::text, nome FROM items")
        .fetch_all(&state.pool).await.unwrap();
    Json(items)
}

async fn list_data(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT * FROM sensor_data ORDER BY ts DESC LIMIT 50")
        .fetch_all(&state.pool).await.unwrap();
    Json(rows.into_iter().map(row_to_json).collect())
}

async fn list_data_by_device(
    State(state): State<AppState>,
    Path(device): Path<String>
) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT * FROM sensor_data WHERE device = $1 ORDER BY ts DESC LIMIT 50")
        .bind(device).fetch_all(&state.pool).await.unwrap();
    Json(rows.into_iter().map(row_to_json).collect())
}

fn row_to_json(row: sqlx::postgres::PgRow) -> serde_json::Value {
    use sqlx::Row;
    serde_json::json!({
        "id": row.try_get::<i32, _>("id").ok(),
        "ts": row.try_get::<chrono::NaiveDateTime, _>("ts").ok(),
        "device": row.try_get::<String, _>("device").ok(),
        "wifi_rssi": row.try_get::<Option<i32>, _>("wifi_rssi").ok(),
        "heading_deg": row.try_get::<Option<f32>, _>("heading_deg").ok(),
        "roll_deg": row.try_get::<Option<f32>, _>("roll_deg").ok(),
        "pitch_deg": row.try_get::<Option<f32>, _>("pitch_deg").ok(),
        "temp_c": row.try_get::<Option<f32>, _>("temp_c").ok(),
        "ultrasonic_cm": row.try_get::<Option<f32>, _>("ultrasonic_cm").ok(),
    })
}

/* ----------------------------- MQTT -> DB + broadcast ----------------------------- */

async fn mqtt_subscribe(state: AppState) -> anyhow::Result<()> {
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri("tcp://test.mosquitto.org:1883")
        .client_id("rust-subscriber-3d")
        .finalize();

    let mut cli = mqtt::AsyncClient::new(create_opts)?;
    cli.connect(None).await?;
    cli.subscribe("devices/esp32/+/state", 1).await?;
    println!("📡 Inscrito no tópico devices/esp32/+/state");

    let mut stream = cli.get_stream(25);

    while let Ok(Some(msg)) = stream.recv().await {
        let payload_str = msg.payload_str().to_string();

        // 1) Broadcast para os clientes WebSocket
        let _ = state.tx.send(payload_str.clone());

        // 2) Persistir no banco
        if let Ok(p) = serde_json::from_str::<SensorPayload>(&payload_str) {
            let q = sqlx::query(
                r#"
            INSERT INTO sensor_data (
                ts, device, wifi_rssi, bno_ok,
                heading_deg, roll_deg, pitch_deg, temp_c,
                accel_x, accel_y, accel_z,
                gyro_x, gyro_y, gyro_z,
                mag_x, mag_y, mag_z,
                calib_sys, calib_gyro, calib_accel, calib_mag,
                ultrasonic_cm
            )
            VALUES (
                TO_TIMESTAMP($1), $2, $3, $4,
                $5, $6, $7, $8,
                $9, $10, $11,
                $12, $13, $14,
                $15, $16, $17,
                $18, $19, $20, $21,
                $22
            )
            "#
            )
                .bind(p.ts)
                .bind(&p.device)
                .bind(p.wifi.as_ref().map(|w| w.rssi))
                .bind(p.bno055.as_ref().map(|b| b.ok))
                .bind(p.bno055.as_ref().and_then(|b| b.heading_deg))
                .bind(p.bno055.as_ref().and_then(|b| b.roll_deg))
                .bind(p.bno055.as_ref().and_then(|b| b.pitch_deg))
                .bind(p.bno055.as_ref().and_then(|b| b.temp_c))
                .bind(p.bno055.as_ref().and_then(|b| b.linear_accel_ms2.as_ref().map(|v| v.x)))
                .bind(p.bno055.as_ref().and_then(|b| b.linear_accel_ms2.as_ref().map(|v| v.y)))
                .bind(p.bno055.as_ref().and_then(|b| b.linear_accel_ms2.as_ref().map(|v| v.z)))
                .bind(p.bno055.as_ref().and_then(|b| b.gyro_rads.as_ref().map(|v| v.x)))
                .bind(p.bno055.as_ref().and_then(|b| b.gyro_rads.as_ref().map(|v| v.y)))
                .bind(p.bno055.as_ref().and_then(|b| b.gyro_rads.as_ref().map(|v| v.z)))
                .bind(p.bno055.as_ref().and_then(|b| b.mag_uT.as_ref().map(|v| v.x)))
                .bind(p.bno055.as_ref().and_then(|b| b.mag_uT.as_ref().map(|v| v.y)))
                .bind(p.bno055.as_ref().and_then(|b| b.mag_uT.as_ref().map(|v| v.z)))
                .bind(p.bno055.as_ref().and_then(|b| b.calib.as_ref().map(|c| c.sys)))
                .bind(p.bno055.as_ref().and_then(|b| b.calib.as_ref().map(|c| c.gyro)))
                .bind(p.bno055.as_ref().and_then(|b| b.calib.as_ref().map(|c| c.accel)))
                .bind(p.bno055.as_ref().and_then(|b| b.calib.as_ref().map(|c| c.mag)))
                .bind(p.ultrasonic_cm);

            if let Err(e) = q.execute(&state.pool).await {
                eprintln!("❌ Erro ao inserir no DB: {:?}", e);
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
    Ok(())
}
