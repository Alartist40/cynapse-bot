"""Local MQTT Voice Bridge for StackChan — Local Mind.

Runs an embedded MQTT broker on 0.0.0.0:1883 and handles XiaoZhi voice dialogue
with Faster-Whisper, Ollama, and Pocket-TTS.
"""

import argparse
import asyncio
from collections import deque
import json
import logging
from pathlib import Path
import time
from typing import Any
import uuid
import yaml

from amqtt.broker import Broker
import paho.mqtt.client as paho_mqtt

from hub.audio import OpusCodec
from hub.stt import STTEngine
from hub.llm import LLMEngine
from hub.tts import TTSEngine

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("localbrain.mqtt_bridge")

MAX_INCOMING_PCM_BYTES = 16000 * 2 * 15  # 15 seconds 16kHz mono 16-bit PCM


class MqttVoiceSession:
    """Manages conversational state for an MQTT-connected StackChan."""

    def __init__(
        self,
        publish_func,
        mode: str,
        stt: STTEngine | None,
        llm: LLMEngine | None,
        tts: TTSEngine | None,
        client_id: str = "stackchan",
    ):
        self.publish_func = publish_func
        self.mode = mode
        self.stt = stt
        self.llm = llm
        self.tts = tts
        self.client_id = client_id

        self.session_id = str(uuid.uuid4())[:8]
        self.codec = OpusCodec()
        self.history: deque = deque(maxlen=6)

        self.is_listening = False
        self.incoming_pcm = bytearray()
        self.active_pipeline_task: asyncio.Task | None = None
        self.speech_stop_time: float = 0.0
        self.audio_frames_sent: int = 0
        self.downstream_topic = "server-device"

    async def send_json(self, data: dict):
        if "session_id" not in data:
            data["session_id"] = self.session_id
        text = json.dumps(data)
        logger.info(f"Sending MQTT JSON to {self.downstream_topic}: {text}")
        await self.publish_func(self.downstream_topic, text.encode("utf-8"))

    async def send_audio_frame(self, frame: bytes) -> bool:
        await self.publish_func(self.downstream_topic, frame)
        self.audio_frames_sent += 1
        if self.audio_frames_sent <= 3:
            await asyncio.sleep(0.005)
        else:
            await asyncio.sleep(0.055)
        return True

    async def send_audio_frames(self, frames: list[bytes]) -> bool:
        for f in frames:
            if not await self.send_audio_frame(f):
                return False
        return True

    async def cancel_active_turn(self):
        self.audio_frames_sent = 0
        if self.active_pipeline_task and not self.active_pipeline_task.done():
            self.active_pipeline_task.cancel()
            try:
                await self.active_pipeline_task
            except asyncio.CancelledError:
                pass
            self.active_pipeline_task = None
            logger.info("Interrupted active turn (barge-in).")
            await self.send_json({"type": "tts", "state": "stop"})

    async def handle_message(self, topic: str, payload: bytes):
        # Check if text JSON or binary audio
        try:
            text = payload.decode("utf-8")
            if text.startswith("{") and text.endswith("}"):
                data = json.loads(text)
                await self.handle_json(data)
                return
        except Exception:
            pass

        # Binary payload -> treat as Opus audio frame if listening
        if self.is_listening:
            if len(self.incoming_pcm) >= MAX_INCOMING_PCM_BYTES:
                return
            try:
                pcm = self.codec.decode_frame(payload)
                self.incoming_pcm.extend(pcm)
            except Exception as e:
                logger.warning(f"Opus decode error: {e}")

    async def handle_json(self, data: dict):
        msg_type = data.get("type")
        logger.info(f"Received MQTT JSON: {data}")

        if msg_type == "hello":
            if "session_id" in data:
                self.session_id = data["session_id"]
            resp = {
                "type": "hello",
                "version": data.get("version", 1),
                "transport": "mqtt",
                "session_id": self.session_id,
                "audio_params": {
                    "format": "opus",
                    "sample_rate": 16000,
                    "channels": 1,
                    "frame_duration": 60,
                },
            }
            await self.send_json(resp)

        elif msg_type == "listen":
            state = data.get("state")
            if state == "start":
                await self.cancel_active_turn()
                self.is_listening = True
                self.incoming_pcm.clear()
                if "session_id" in data:
                    self.session_id = data["session_id"]
                logger.info(f"Listen start (session {self.session_id})")

            elif state == "stop":
                self.is_listening = False
                self.speech_stop_time = time.time()
                pcm_snapshot = bytes(self.incoming_pcm)
                self.incoming_pcm.clear()
                logger.info(f"Listen stop. Recorded {len(pcm_snapshot)} bytes PCM.")
                self.active_pipeline_task = asyncio.create_task(
                    self.execute_turn(pcm_snapshot, self.speech_stop_time)
                )

        elif msg_type == "abort":
            await self.cancel_active_turn()

    async def execute_turn(self, pcm_data: bytes, speech_stop_time: float):
        if not pcm_data:
            return
        if self.mode == "echo":
            await self._execute_echo_turn(pcm_data)
        else:
            await self._execute_full_turn(pcm_data, speech_stop_time)

    async def _execute_echo_turn(self, pcm_data: bytes):
        frames = self.codec.pcm_to_opus_frames(pcm_data)
        await self.send_json({"type": "tts", "state": "start"})
        await self.send_json({"type": "tts", "state": "sentence_start", "text": "echo"})
        await self.send_audio_frames(frames)
        await self.send_json({"type": "tts", "state": "sentence_end"})
        await self.send_json({"type": "tts", "state": "stop"})

    async def _execute_full_turn(self, pcm_data: bytes, speech_stop_time: float):
        if not self.stt or not self.llm or not self.tts:
            return

        t0 = time.time()
        transcript = await asyncio.to_thread(self.stt.transcribe_pcm, pcm_data)
        logger.info(f"STT: '{transcript}' ({(time.time()-t0)*1000:.1f}ms)")
        if not transcript.strip():
            return

        await self.send_json({"type": "tts", "state": "start"})
        self.audio_frames_sent = 0
        first_frame_sent = False

        full_reply_chunks = []
        async for chunk in self.llm.generate_response_stream(transcript, list(self.history)):
            full_reply_chunks.append(chunk)
            await self.send_json({"type": "tts", "state": "sentence_start", "text": chunk})

            async for opus_frame in self.tts.stream_speech_opus_frames(chunk):
                if not first_frame_sent:
                    first_frame_sent = True
                    logger.info(f"Time to first audio: {(time.time() - speech_stop_time)*1000:.1f}ms")
                if not await self.send_audio_frame(opus_frame):
                    break
            await self.send_json({"type": "tts", "state": "sentence_end"})

        full_reply = "".join(full_reply_chunks)
        self.history.append({"role": "user", "content": transcript})
        self.history.append({"role": "assistant", "content": full_reply})
        await self.send_json({"type": "tts", "state": "stop"})


class LocalMqttBridge:
    def __init__(self, config_path: str = "hub/config.yaml", mode_override: str | None = None):
        with open(config_path, "r") as f:
            self.config = yaml.safe_load(f)

        vh_cfg = self.config.get("voice_hub", {})
        self.mode = mode_override or vh_cfg.get("mode", "echo")

        if self.mode == "full":
            stt_cfg = self.config.get("stt", {})
            llm_cfg = self.config.get("llm", {})
            tts_cfg = self.config.get("tts", {})
            self.stt = STTEngine(model_size=stt_cfg.get("model_size", "small"), language="en")
            self.llm = LLMEngine(
                host=llm_cfg.get("host", "http://127.0.0.1:11434"),
                model=llm_cfg.get("model", "qwen2.5:3b-instruct"),
                system_prompt_path=llm_cfg.get("system_prompt_path", "hub/persona/system.md"),
            )
            self.tts = TTSEngine(server_url=tts_cfg.get("url", "http://127.0.0.1:8000"))
        else:
            self.stt = None
            self.llm = None
            self.tts = None

        self.session: MqttVoiceSession | None = None
        self.loop = asyncio.get_event_loop()

    async def publish_mqtt(self, topic: str, payload: bytes):
        self.mqtt_client.publish(topic, payload)

    def on_message(self, client, userdata, msg):
        topic = msg.topic
        payload = msg.payload
        if self.session:
            asyncio.run_coroutine_threadsafe(self.session.handle_message(topic, payload), self.loop)

    async def start(self):
        # 1. Start Broker
        broker_cfg = {
            "listeners": {"default": {"type": "tcp", "bind": "0.0.0.0:1883"}},
            "sys_interval": 0,
            "auth": {"allow-anonymous": True, "plugins": ["auth_anonymous"]},
        }
        self.broker = Broker(broker_cfg)
        await self.broker.start()
        logger.info("Local MQTT broker started on 0.0.0.0:1883")

        # 2. Start internal client
        self.session = MqttVoiceSession(
            publish_func=self.publish_mqtt,
            mode=self.mode,
            stt=self.stt,
            llm=self.llm,
            tts=self.tts,
        )

        self.mqtt_client = paho_mqtt.Client(paho_mqtt.CallbackAPIVersion.VERSION2, client_id="localmind-hub")
        self.mqtt_client.on_message = self.on_message
        self.mqtt_client.connect("127.0.0.1", 1883, 60)
        self.mqtt_client.subscribe("#")
        self.mqtt_client.loop_start()
        logger.info(f"LocalMind Voice Bridge listening on MQTT (mode={self.mode})...")

        await asyncio.Future()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["echo", "full"], default="echo")
    args = parser.parse_args()

    bridge = LocalMqttBridge(mode_override=args.mode)
    asyncio.run(bridge.start())


if __name__ == "__main__":
    main()
