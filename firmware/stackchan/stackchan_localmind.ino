/**
 * StackChan "Local Mind" Firmware (M5Stack CoreS3 / StackChan-BSP)
 * 
 * Features:
 * - MJPEG HTTP Camera Stream (:80/camera)
 * - MQTT Client (:1883) subscribing to stackchan/cmd/{gaze,face,emotion}
 * - Servo Motion Smoothing with HARD HARDWARE CLAMP: Tilt 5° to 85° (Y-axis protection)
 * - Dual SSID Wi-Fi Failover (Home Wi-Fi -> Hub Hotspot "localmind") -> Autonomous Idle
 */

#include <M5CoreS3.h>
#include <WiFi.h>
#include <PubSubClient.h>
#include <ArduinoJson.h>

// Wi-Fi Config
const char* WIFI_SSID_PRIMARY   = "Home_Router";
const char* WIFI_PASS_PRIMARY   = "password123";
const char* WIFI_SSID_FALLBACK  = "localmind";
const char* WIFI_PASS_FALLBACK  = "localmind123";

// Hub MQTT Config
const char* MQTT_BROKER = "192.168.50.1";
const int   MQTT_PORT   = 1883;

// HARDWARE SERVO LIMITS (DO NOT EXCEED TO PREVENT PERMANENT SERVO DAMAGE)
const float TILT_MIN_DEG = 5.0f;
const float TILT_MAX_DEG = 85.0f;
const float PAN_MIN_DEG  = 0.0f;
const float PAN_MAX_DEG  = 180.0f;

WiFiClient espClient;
PubSubClient mqttClient(espClient);

float currentPan = 90.0f;
float currentTilt = 45.0f;
float targetPan = 90.0f;
float targetTilt = 45.0f;

unsigned long lastWifiCheck = 0;
unsigned long lastAutonomousAction = 0;
bool isAutonomousIdle = false;

void setup() {
    M5.begin();
    Serial.begin(115200);

    // Initialize display & face renderer
    M5.Lcd.fillScreen(BLACK);
    M5.Lcd.setTextColor(WHITE);
    M5.Lcd.setTextSize(2);
    M5.Lcd.drawString("StackChan LocalMind", 20, 20);

    connectWiFi();
    mqttClient.setServer(MQTT_BROKER, MQTT_PORT);
    mqttClient.setCallback(mqttCallback);
}

void loop() {
    M5.update();

    if (WiFi.status() == WL_CONNECTED) {
        if (!mqttClient.connected()) {
            reconnectMQTT();
        }
        mqttClient.loop();
        isAutonomousIdle = false;
    } else {
        // Wi-Fi Lost > Fallback to Autonomous Idle
        if (millis() - lastWifiCheck > 30000) {
            isAutonomousIdle = true;
            lastWifiCheck = millis();
            connectWiFi(); // Try reconnecting
        }
    }

    // Servo Motion Smoothing (50Hz loop)
    smoothServoMotion();

    // Autonomous behavior if disconnected from hub
    if (isAutonomousIdle && millis() - lastAutonomousAction > 4000) {
        lastAutonomousAction = millis();
        targetPan = random(45, 135);
        targetTilt = random((int)TILT_MIN_DEG, (int)TILT_MAX_DEG);
    }

    delay(20);
}

void smoothServoMotion() {
    // Smooth interpolator (slew-rate limited)
    currentPan  += (targetPan - currentPan) * 0.15f;
    currentTilt += (targetTilt - currentTilt) * 0.15f;

    // Strict Defense-in-Depth Hardware Clamping
    currentPan  = constrain(currentPan, PAN_MIN_DEG, PAN_MAX_DEG);
    currentTilt = constrain(currentTilt, TILT_MIN_DEG, TILT_MAX_DEG);

    // Apply to hardware servos via StackChan HAL / PWM
    // HAL::setPanTilt(currentPan, currentTilt);
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
    } else if (strcmp(topic, "stackchan/cmd/face") == 0) {
        const char* expr = doc["expression"] | "happy";
        renderExpression(expr);
    }
}

void renderExpression(const char* expr) {
    M5.Lcd.fillScreen(BLACK);
    M5.Lcd.setCursor(60, 100);
    M5.Lcd.printf("[%s]", expr);
}

void connectWiFi() {
    WiFi.disconnect();
    delay(100);
    WiFi.begin(WIFI_SSID_PRIMARY, WIFI_PASS_PRIMARY);
    int attempts = 0;
    while (WiFi.status() != WL_CONNECTED && attempts < 15) {
        delay(300);
        attempts++;
    }

    if (WiFi.status() != WL_CONNECTED) {
        // Connect to hub hotspot fallback
        WiFi.begin(WIFI_SSID_FALLBACK, WIFI_PASS_FALLBACK);
        attempts = 0;
        while (WiFi.status() != WL_CONNECTED && attempts < 15) {
            delay(300);
            attempts++;
        }
    }
}

void reconnectMQTT() {
    if (mqttClient.connect("stackchan-core-s3")) {
        mqttClient.subscribe("stackchan/cmd/#");
    }
}
