/**
 * StackChan "Local Mind" Firmware (M5Stack CoreS3)
 * 
 * Features:
 * - MQTT Client subscribing to stackchan/cmd/{gaze,face,emotion}
 * - Real Servo PWM output with HARD HARDWARE CLAMP: Tilt 5.0° to 85.0° (Y-axis protection)
 * - Non-blocking Wi-Fi State Machine (Home Wi-Fi -> Hub Hotspot "localmind" -> Autonomous Idle)
 */

#include <M5CoreS3.h>
#include <WiFi.h>
#include <PubSubClient.h>
#include <ArduinoJson.h>
#include <ESP32Servo.h>

#if __has_include("config.h")
  #include "config.h"
#else
  #warning "config.h not found! Copy firmware/stackchan/config.h.example -> config.h to configure Wi-Fi and MQTT credentials."
  #define WIFI_SSID_PRIMARY   ""
  #define WIFI_PASS_PRIMARY   ""
  #define WIFI_SSID_FALLBACK  "localmind"
  #define WIFI_PASS_FALLBACK  ""

  #define MQTT_BROKER "192.168.50.1"
  #define MQTT_PORT   1883
  #define MQTT_USER   ""
  #define MQTT_PASS   ""
#endif

// Servo Pins (StackChan CoreS3 standard PWM pins)


const int SERVO_PAN_PIN  = 1;
const int SERVO_TILT_PIN = 2;

// HARDWARE SERVO LIMITS (DO NOT EXCEED TO PREVENT PERMANENT SERVO DAMAGE)
const float TILT_MIN_DEG = 5.0f;
const float TILT_MAX_DEG = 85.0f;
const float PAN_MIN_DEG  = 0.0f;
const float PAN_MAX_DEG  = 180.0f;

Servo servoPan;
Servo servoTilt;

WiFiClient espClient;
PubSubClient mqttClient(espClient);

float currentPan = 90.0f;
float currentTilt = 45.0f;
float targetPan = 90.0f;
float targetTilt = 45.0f;

enum WifiState {
    WIFI_DISCONNECTED,
    WIFI_CONNECTING_PRIMARY,
    WIFI_CONNECTING_FALLBACK,
    WIFI_CONNECTED
};

WifiState currentWifiState = WIFI_DISCONNECTED;
unsigned long wifiStateTimer = 0;
unsigned long lastMqttRetry = 0;
unsigned long lastAutonomousAction = 0;
bool isAutonomousIdle = true;

void setup() {
    M5.begin();
    Serial.begin(115200);

    // Initialize servos
    ESP32PWM::allocateTimer(0);
    ESP32PWM::allocateTimer(1);
    servoPan.setPeriodHertz(50);
    servoTilt.setPeriodHertz(50);
    servoPan.attach(SERVO_PAN_PIN, 500, 2500);
    servoTilt.attach(SERVO_TILT_PIN, 500, 2500);

    servoPan.write((int)currentPan);
    servoTilt.write((int)currentTilt);

    // Initialize display & face renderer
    M5.Lcd.fillScreen(BLACK);
    M5.Lcd.setTextColor(WHITE);
    M5.Lcd.setTextSize(2);
    M5.Lcd.drawString("StackChan LocalMind", 20, 20);

    mqttClient.setServer(MQTT_BROKER, MQTT_PORT);
    mqttClient.setCallback(mqttCallback);

    startWifiConnect(WIFI_CONNECTING_PRIMARY);
}

void loop() {
    M5.update();

    // 1. Non-blocking Wi-Fi State Machine
    updateWifiStateMachine();

    // 2. MQTT loop if connected
    if (currentWifiState == WIFI_CONNECTED) {
        if (!mqttClient.connected()) {
            if (millis() - lastMqttRetry > 3000) {
                lastMqttRetry = millis();
                reconnectMQTT();
            }
        } else {
            mqttClient.loop();
            isAutonomousIdle = false;
        }
    } else {
        isAutonomousIdle = true;
    }

    // 3. Servo Motion Smoothing (50Hz loop)
    smoothServoMotion();

    // 4. Autonomous behavior if disconnected from hub
    if (isAutonomousIdle && millis() - lastAutonomousAction > 4000) {
        lastAutonomousAction = millis();
        targetPan = random(60, 120);
        targetTilt = random((int)TILT_MIN_DEG, (int)TILT_MAX_DEG);
    }

    delay(20);
}

void smoothServoMotion() {
    // Slew-rate limited interpolation
    currentPan  += (targetPan - currentPan) * 0.15f;
    currentTilt += (targetTilt - currentTilt) * 0.15f;

    // Strict Defense-in-Depth Hardware Clamping
    currentPan  = constrain(currentPan, PAN_MIN_DEG, PAN_MAX_DEG);
    currentTilt = constrain(currentTilt, TILT_MIN_DEG, TILT_MAX_DEG);

    // Apply directly to hardware servos
    servoPan.write((int)currentPan);
    servoTilt.write((int)currentTilt);
}

void startWifiConnect(WifiState targetState) {
    currentWifiState = targetState;
    wifiStateTimer = millis();

    if (targetState == WIFI_CONNECTING_PRIMARY) {
        WiFi.disconnect();
        WiFi.begin(WIFI_SSID_PRIMARY, WIFI_PASS_PRIMARY);
    } else if (targetState == WIFI_CONNECTING_FALLBACK) {
        WiFi.disconnect();
        WiFi.begin(WIFI_SSID_FALLBACK, WIFI_PASS_FALLBACK);
    }
}

void updateWifiStateMachine() {
    if (currentWifiState == WIFI_CONNECTED) {
        if (WiFi.status() != WL_CONNECTED) {
            startWifiConnect(WIFI_CONNECTING_PRIMARY);
        }
        return;
    }

    if (WiFi.status() == WL_CONNECTED) {
        currentWifiState = WIFI_CONNECTED;
        Serial.println("Wi-Fi Connected.");
        return;
    }

    if (currentWifiState == WIFI_CONNECTING_PRIMARY) {
        if (millis() - wifiStateTimer > 6000) {
            // Primary timed out, try fallback
            startWifiConnect(WIFI_CONNECTING_FALLBACK);
        }
    } else if (currentWifiState == WIFI_CONNECTING_FALLBACK) {
        if (millis() - wifiStateTimer > 6000) {
            // Fallback timed out, return to primary retry
            startWifiConnect(WIFI_CONNECTING_PRIMARY);
        }
    }
}

void mqttCallback(char* topic, byte* payload, unsigned int length) {
    StaticJsonDocument<256> doc;
    DeserializationError error = deserializeJson(doc, payload, length);
    if (error) return;

    if (strcmp(topic, "stackchan/cmd/gaze") == 0) {
        if (doc.containsKey("pan_angle") && doc.containsKey("tilt_angle")) {
            float rawPan = doc["pan_angle"];
            float rawTilt = doc["tilt_angle"];

            targetPan = constrain(rawPan, PAN_MIN_DEG, PAN_MAX_DEG);
            targetTilt = constrain(rawTilt, TILT_MIN_DEG, TILT_MAX_DEG); // Strict clamp
        }
    } else if (strcmp(topic, "stackchan/cmd/face") == 0 || strcmp(topic, "stackchan/cmd/emotion") == 0) {
        const char* expr = doc["expression"] | doc["emotion"] | "happy";
        renderExpression(expr);
    } else if (strcmp(topic, "stackchan/cmd/audio") == 0 || strcmp(topic, "stackchan/cmd/speak") == 0) {
        // Speaker playback trigger (shows talking indicator on display)
        renderExpression("talking");
    }
}

void renderExpression(const char* expr) {
    M5.Lcd.fillScreen(BLACK);
    M5.Lcd.setCursor(60, 100);
    M5.Lcd.printf("[%s]", expr);
}

void reconnectMQTT() {
    uint64_t chipid = ESP.getEfuseMac();
    char clientId[32];
    snprintf(clientId, sizeof(clientId), "stackchan-%04X%08X", (uint16_t)(chipid >> 32), (uint32_t)chipid);

    #if defined(MQTT_USER) && defined(MQTT_PASS)
    bool connected = mqttClient.connect(clientId, MQTT_USER, MQTT_PASS);
    #else
    bool connected = mqttClient.connect(clientId);
    #endif

    if (connected) {
        Serial.printf("MQTT connected as %s\n", clientId);
        mqttClient.subscribe("stackchan/cmd/#");
    }
}

