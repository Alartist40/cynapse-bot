# CynapseBot Mobile Companion

This directory contains the standalone mobile wrapper and configuration for running the CynapseBot companion app on iOS and Android devices.

## 1. Instant PWA Mode (Recommended - No Compilation Needed)
1. Ensure your Orange Pi / PC running `cynpase-bot` and your phone are on the same Wi-Fi network.
2. Open your mobile browser (Safari on iOS or Chrome on Android) and navigate to:
   ```
   http://<YOUR_OPI_IP>:8000/mobile
   ```
3. Tap the browser share/menu button and select **"Add to Home Screen"**.
4. The CynapseBot companion icon will now appear on your phone home screen with full-screen native capability, haptic touch feedback, virtual joystick, and canvas avatar!

---

## 2. Native Android APK Packaging (Capacitor / WebView)

If you wish to compile a standalone `.apk`:

```bash
cd mobile
npm install @capacitor/core @capacitor/cli @capacitor/android
npx cap init CynapseBot com.cynapse.bot --web-dir src
npx cap add android
npx cap open android
```

In `capacitor.config.json`, configure the server URL to point to your robot hub:
```json
{
  "appId": "com.cynapse.bot",
  "appName": "CynapseBot",
  "webDir": "src",
  "server": {
    "url": "http://192.168.1.100:8000/mobile",
    "cleartext": true
  }
}
```

---

## Mobile Features

- 🕹️ **Touch Joystick**: 360° pan (-90° to +90°) and tilt (-30° to +30°) servo control.
- 🤖 **StackChan Screen Mode**: Animated HTML5 canvas with cute blinking eyes, emotional expressions, and eye gaze tracking.
- 💬 **Live Voice & Chat Feed**: Push-to-talk microphone bridge and real-time LLM dialogue.
- 💃 **Choreography Launcher**: Dance, nod, wave, and RGB LED color palettes.
- 👁️ **Camera Viewfinder**: 0.3MP real-time JPEG frame monitor.
- 🌌 **Mazzaroth 3D Galaxy**: Interactive memory constellation cosmos.
