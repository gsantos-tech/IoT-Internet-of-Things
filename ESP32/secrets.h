#pragma once

// ===== WIFI =====
#define WIFI_SSID     "AMF"
#define WIFI_PASSWORD "amf@2025"

// ===== MQTT =====
#define MQTT_HOST     "test.mosquitto.org"   // ex: "192.168.1.10" ou "broker.emqx.io"
#define MQTT_PORT     1883                // 1883 (TCP). Se usar WebSocket, use outra lib (não PubSubClient).
#define MQTT_USER     ""                  // opcional
#define MQTT_PASS     ""                  // opcional

// Tópico base: personalize como preferir
#define MQTT_BASE_TOPIC "devices/esp32"
