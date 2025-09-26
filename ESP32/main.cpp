#include <Arduino.h>
#include <WiFi.h>
#include <Wire.h>
#include <PubSubClient.h>
#include <ArduinoJson.h>
#include <Adafruit_BNO055.h>
#include <Adafruit_Sensor.h>
#include "secrets.h"

// ----------------- Pinos -----------------
#define SDA_PIN   19
#define SCL_PIN   18
#define TRIG_PIN  25
#define ECHO_PIN  26   // ENTRADA: usar divisor resistivo 5V->3V3

// ----------------- Objetos globais -----------------
Adafruit_BNO055 bno(55, 0x28, &Wire);   // 0x28 (ADR GND). Use 0x29 se ADR=3V3.
bool bnoOk = false;

WiFiClient wifiClient;
PubSubClient mqtt(wifiClient);

// ----------------- Tempo/NTP -----------------
const long gmtOffset_sec      = -3 * 3600; // America/Sao_Paulo
const int  daylightOffset_sec = 0;

// ----------------- Identidade -----------------
String deviceId;

// ----------------- Utilidades -----------------
String chipIdHex() {
  uint64_t mac = ESP.getEfuseMac();
  char buf[17];
  snprintf(buf, sizeof(buf), "%04X%08X",
           (uint16_t)(mac >> 32), (uint32_t)(mac & 0xFFFFFFFF));
  return String(buf);
}

void connectWiFi() {
  if (WiFi.status() == WL_CONNECTED) return;
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
  Serial.print(F("Conectando ao Wi-Fi"));
  unsigned long start = millis();
  while (WiFi.status() != WL_CONNECTED) {
    Serial.print('.');
    delay(500);
    if (millis() - start > 20000) { // timeout 20s
      Serial.println(F("\nTimeout Wi-Fi, tentando novamente..."));
      start = millis();
      WiFi.disconnect(true);
      WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
    }
  }
  Serial.printf("\nWi-Fi OK. IP: %s\n", WiFi.localIP().toString().c_str());
}

void ensureNTP() {
  static bool configured = false;
  if (!configured) {
    configTime(gmtOffset_sec, daylightOffset_sec, "pool.ntp.org", "time.nist.gov");
    configured = true;
  }
}

time_t nowEpoch() {
  time_t now;
  time(&now);
  return now;
}

bool initBNO() {
  Wire.begin(SDA_PIN, SCL_PIN);
  delay(10);
  if (!bno.begin()) {
    Serial.println(F("BNO055 não detectado (verifique fios/endereço)."));
    return false;
  }
  bno.setExtCrystalUse(true);
  delay(20);
  return true;
}

// Medição HC-SR04: mediana de 5 leituras para robustez
float measureDistanceCm() {
  const int N = 5;
  float vals[N];

  for (int i = 0; i < N; i++) {
    digitalWrite(TRIG_PIN, LOW);
    delayMicroseconds(4);
    digitalWrite(TRIG_PIN, HIGH);
    delayMicroseconds(10);
    digitalWrite(TRIG_PIN, LOW);

    // timeout 30ms ~ 5m
    unsigned long dur = pulseIn(ECHO_PIN, HIGH, 30000UL);
    float cm = (dur == 0) ? NAN : (dur / 58.0f);
    vals[i] = cm;
    delay(30);
  }

  // insertion sort simples (trata NaN como "maior")
  for (int i = 1; i < N; i++) {
    float key = vals[i];
    int j = i - 1;
    while (j >= 0 && ( (isnan(vals[j]) && !isnan(key)) || (!isnan(vals[j]) && !isnan(key) && vals[j] > key) )) {
      vals[j + 1] = vals[j];
      j--;
    }
    vals[j + 1] = key;
  }
  return vals[N / 2];
}

void connectMQTT() {
  mqtt.setServer(MQTT_HOST, MQTT_PORT);
  mqtt.setKeepAlive(30);
  mqtt.setBufferSize(1024); // payload confortável

  String clientId = "esp32-" + deviceId;

  // Last Will & Testament (LWT)
  String willTopic = String(MQTT_BASE_TOPIC) + "/" + deviceId + "/status";
  const char* willMsg = "offline";

  while (!mqtt.connected()) {
    Serial.print(F("Conectando ao MQTT... "));
    bool ok = false;
    if (strlen(MQTT_USER) > 0) {
      ok = mqtt.connect(clientId.c_str(), MQTT_USER, MQTT_PASS,
                        willTopic.c_str(), 1, true, willMsg);
    } else {
      ok = mqtt.connect(clientId.c_str(),
                        willTopic.c_str(), 1, true, willMsg);
    }
    if (ok) {
      Serial.println(F("conectado!"));
      // publica "online" como retained
      mqtt.publish(willTopic.c_str(), "online", true);
    } else {
      Serial.printf("falhou (rc=%d). Tentando em 3s...\n", mqtt.state());
      delay(3000);
    }
  }
}

void publishSensors() {
  // -------- JSON v7 --------
  JsonDocument doc;
  doc["ts"] = (long)nowEpoch();
  doc["device"] = deviceId;

  JsonObject wifi = doc["wifi"].to<JsonObject>();
  wifi["rssi"] = WiFi.RSSI();

  // -------- BNO055 --------
  JsonObject bnoj = doc["bno055"].to<JsonObject>();
  bnoj["ok"] = bnoOk;

  if (bnoOk) {
    sensors_event_t ori, acc, gyro, mag;
    bno.getEvent(&ori, Adafruit_BNO055::VECTOR_EULER);        // yaw, roll, pitch (graus)
    bno.getEvent(&acc, Adafruit_BNO055::VECTOR_LINEARACCEL);  // m/s²
    bno.getEvent(&gyro, Adafruit_BNO055::VECTOR_GYROSCOPE);   // rad/s
    bno.getEvent(&mag, Adafruit_BNO055::VECTOR_MAGNETOMETER); // uT
    float tempC = bno.getTemp();

    bnoj["heading_deg"] = ori.orientation.x;  // yaw
    bnoj["roll_deg"]    = ori.orientation.z;
    bnoj["pitch_deg"]   = ori.orientation.y;
    bnoj["temp_c"]      = tempC;

    JsonObject lin = bnoj["linear_accel_ms2"].to<JsonObject>();
    lin["x"] = acc.acceleration.x;
    lin["y"] = acc.acceleration.y;
    lin["z"] = acc.acceleration.z;

    JsonObject gry = bnoj["gyro_rads"].to<JsonObject>();
    gry["x"] = gyro.gyro.x;
    gry["y"] = gyro.gyro.y;
    gry["z"] = gyro.gyro.z;

    JsonObject mg = bnoj["mag_uT"].to<JsonObject>();
    mg["x"] = mag.magnetic.x;
    mg["y"] = mag.magnetic.y;
    mg["z"] = mag.magnetic.z;

    uint8_t sys, g, a, m;
    bno.getCalibration(&sys, &g, &a, &m);
    JsonObject cal = bnoj["calib"].to<JsonObject>();
    cal["sys"]   = sys;
    cal["gyro"]  = g;
    cal["accel"] = a;
    cal["mag"]   = m;
  }

  // -------- Ultrassônico --------
  float dist_cm = measureDistanceCm();
  // JSON não tem NaN; só adiciona se for válido
  if (!isnan(dist_cm)) doc["ultrasonic_cm"] = dist_cm;

  // -------- Publica --------
  char payload[1024];
  size_t n = serializeJson(doc, payload, sizeof(payload));

  String topic = String(MQTT_BASE_TOPIC) + "/" + deviceId + "/state";
  if (!mqtt.publish(topic.c_str(), (uint8_t*)payload, n)) {
    Serial.println(F("Falha ao publicar MQTT."));
  } else {
    Serial.print(F("Publicado em "));
    Serial.print(topic);
    Serial.print(F(": "));
    Serial.println(payload);
  }
}

void setup() {
  Serial.begin(115200);
  delay(200);

  pinMode(TRIG_PIN, OUTPUT);
  pinMode(ECHO_PIN, INPUT);

  deviceId = chipIdHex();
  Serial.printf("Device ID: %s\n", deviceId.c_str());

  connectWiFi();
  ensureNTP();

  bnoOk = initBNO();
  if (!bnoOk) {
    Serial.println(F("Prosseguindo sem BNO055 (bnoOk=false)."));
  }

  connectMQTT();
}

unsigned long lastPub = 0;

void loop() {
  // mantém conexões
  if (WiFi.status() != WL_CONNECTED) connectWiFi();
  if (!mqtt.connected()) connectMQTT();
  mqtt.loop();

  // publica a cada 1s
  unsigned long now = millis();
  if (now - lastPub >= 1000) {
    publishSensors();
    lastPub = now;
  }
}