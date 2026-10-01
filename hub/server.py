"""XiaoZhi Protocol WebSocket Server for StackChan — LocalBrain Hub."""

import argparse
import asyncio
from collections import deque
import json
import logging
import os
from pathlib import Path
import time
from typing import Any
import uuid
import websockets
import yaml

from hub.audio import OpusCodec
from hub.stt import STTEngine
from hub.llm import LLMEngine
from hub.tts import TTSEngine

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("localbrain.hub")


class LocalBrainSession:
    """Manages the state and conversational pipeline for a single StackChan connection."""

    def __init__(
        self,
        ws: Any,
        mode: str,
        stt: STTEngine | None,
        llm: LLMEngine | None,
        tts: TTSEngine | None,
        protocol_log_path: Path | None,
        latency_log_path: Path | None,
    ):
        self.ws = ws
        self.mode = mode
        self.stt = stt
        self.llm = llm
        self.tts = tts
        self.protocol_log_path = protocol_log_path
        self.latency_log_path = latency_log_path

        self.session_id = str(uuid.uuid4())
        self.codec = OpusCodec()
        self.history: deque = deque(maxlen=6)

        self.is_listening = False
        self.incoming_pcm = bytearray()
        self.active_pipeline_task: asyncio.Task | None = None
        self.speech_stop_time: float = 0.0

    async def log_protocol(self, direction: str, payload: dict | str):
        if not self.protocol_log_path:
            return
        entry = {
            "timestamp": time.time(),
            "session_id": self.session_id,
            "direction": direction,
            "payload": payload if isinstance(payload, dict) else {"raw": payload},
        }
        with open(self.protocol_log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry) + "\n")

    def log_latency(self, transcript: str, speech_to_audio_latency_ms: float):
        if not self.latency_log_path:
            return
        entry = {
            "timestamp": time.time(),
            "session_id": self.session_id,
            "transcript": transcript,
            "speech_to_first_audio_ms": round(speech_to_audio_latency_ms, 2),
        }
        with open(self.latency_log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry) + "\n")

    async def send_json(self, data: dict):
        text = json.dumps(data)
        await self.log_protocol("server->client", data)
        await self.ws.send(text)

    async def send_audio_frames(self, frames: list[bytes]):
        for frame in frames:
            await self.ws.send(frame)
            await asyncio.sleep(0.005)

    async def cancel_active_turn(self):
        """Barge-in interruption: cancel pending LLM and TTS tasks immediately."""
        if self.active_pipeline_task and not self.active_pipeline_task.done():
            self.active_pipeline_task.cancel()
            try:
                await self.active_pipeline_task
            except asyncio.CancelledError:
                pass
            self.active_pipeline_task = None
            logger.info("Interrupted current turn (barge-in triggered).")
            await self.send_json({"type": "tts", "state": "stop"})

    async def handle_text_frame(self, message: str):
        try:
            data = json.loads(message)
        except Exception as e:
            logger.warning(f"Failed to parse text frame JSON: {e}")
            return

        await self.log_protocol("client->server", data)
        msg_type = data.get("type")

        if msg_type == "hello":
            logger.info(f"Handshake hello from client (session {self.session_id})")
            resp = {
                "type": "hello",
                "transport": "websocket",
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
                # Interruption handling
                await self.cancel_active_turn()
                self.is_listening = True
                self.incoming_pcm.clear()
                logger.info("Listen started, recording audio.")

            elif state == "stop":
                self.is_listening = False
                self.speech_stop_time = time.time()
                logger.info(f"Listen stopped. Recorded {len(self.incoming_pcm)} bytes PCM.")
                pcm_snapshot = bytes(self.incoming_pcm)
                self.incoming_pcm.clear()
                self.active_pipeline_task = asyncio.create_task(
                    self.execute_turn(pcm_snapshot, self.speech_stop_time)
                )

        elif msg_type == "abort":
            await self.cancel_active_turn()

    async def handle_binary_frame(self, opus_bytes: bytes):
        if self.is_listening:
            try:
                pcm_frame = self.codec.decode_frame(opus_bytes)
                self.incoming_pcm.extend(pcm_frame)
            except Exception as e:
                logger.warning(f"Error decoding incoming Opus frame: {e}")

    async def execute_turn(self, pcm_data: bytes, speech_stop_time: float):
        """Execute either Echo turn or full STT->LLM->TTS pipeline turn."""
        try:
            if self.mode == "echo":
                await self._execute_echo_turn(pcm_data)
            else:
                await self._execute_full_turn(pcm_data, speech_stop_time)
        except asyncio.CancelledError:
            logger.info("Turn execution cancelled.")
            raise
        except Exception as e:
            logger.exception(f"Error during turn execution: {e}")
            await self.send_json({"type": "tts", "state": "stop"})

    async def _execute_echo_turn(self, pcm_data: bytes):
        if not pcm_data:
            return

        frames = self.codec.pcm_to_opus_frames(pcm_data)
        await self.send_json({"type": "tts", "state": "start"})
        await self.send_json({"type": "tts", "state": "sentence_start", "text": "echo"})
        await self.send_audio_frames(frames)
        await self.send_json({"type": "tts", "state": "sentence_end"})
        await self.send_json({"type": "tts", "state": "stop"})

    async def _execute_full_turn(self, pcm_data: bytes, speech_stop_time: float):
        if not pcm_data or self.stt is None or self.llm is None or self.tts is None:
            return

        # 1. STT Transcription
        t0 = time.time()
        transcript = self.stt.transcribe_pcm(pcm_data)
        stt_latency = (time.time() - t0) * 1000.0
        logger.info(f"STT Transcript ({stt_latency:.1f}ms): {transcript}")

        if not transcript.strip():
            return

        await self.send_json({"type": "stt", "text": transcript})

        # 2. LLM + TTS Streaming
        await self.send_json({"type": "tts", "state": "start"})

        full_reply_parts = []
        first_audio_sent = False

        async for sentence in self.llm.stream_reply_sentences(transcript, self.history):
            if not sentence.strip():
                continue
            full_reply_parts.append(sentence)
            await self.send_json({"type": "llm", "text": sentence})

            # TTS Audio Generation for this sentence
            opus_frames = await self.tts.generate_speech_opus_frames(sentence)

            if opus_frames:
                if not first_audio_sent:
                    first_audio_sent = True
                    speech_to_first_audio = (time.time() - speech_stop_time) * 1000.0
                    self.log_latency(transcript, speech_to_first_audio)
                    logger.info(f"First audio latency: {speech_to_first_audio:.1f}ms")

                await self.send_json({"type": "tts", "state": "sentence_start", "text": sentence})
                await self.send_audio_frames(opus_frames)
                await self.send_json({"type": "tts", "state": "sentence_end"})

        # Record exchange in history
        full_reply = " ".join(full_reply_parts).strip()
        if full_reply:
            self.history.append({"role": "user", "content": transcript})
            self.history.append({"role": "assistant", "content": full_reply})

        await self.send_json({"type": "tts", "state": "stop"})


class LocalBrainHub:
    """LocalBrain Hub Server."""

    def __init__(self, config_path: str = "hub/config.yaml", mode_override: str | None = None):
        self.config_path = config_path
        self.config = self._load_config(config_path)

        self.mode = mode_override or self.config.get("server", {}).get("mode", "full")
        self.host = self.config.get("server", {}).get("host", "0.0.0.0")
        self.port = self.config.get("server", {}).get("port", 8100)
        self.endpoint_path = self.config.get("server", {}).get("endpoint_path", "/xiaozhi/v1/")

        self.docs_dir = Path("docs")
        self.docs_dir.mkdir(parents=True, exist_ok=True)
        self.protocol_log = self.docs_dir / "protocol-log.jsonl"
        self.latency_log = self.docs_dir / "latency-log.jsonl"

        # Initialize intelligence engines if in full mode
        if self.mode == "full":
            stt_cfg = self.config.get("stt", {})
            llm_cfg = self.config.get("llm", {})
            tts_cfg = self.config.get("tts", {})

            self.stt = STTEngine(
                model_size=stt_cfg.get("model_size", "small"),
                device=stt_cfg.get("device", "auto"),
                compute_type=stt_cfg.get("compute_type", "int8"),
                language=stt_cfg.get("language", "en"),
            )
            self.llm = LLMEngine(
                host=llm_cfg.get("host", "http://127.0.0.1:11434"),
                model=llm_cfg.get("model", "qwen2.5:3b-instruct"),
                system_prompt_path=llm_cfg.get("system_prompt_path", "hub/persona/system.md"),
                temperature=llm_cfg.get("temperature", 0.7),
            )
            self.tts = TTSEngine(
                server_url=tts_cfg.get("url", "http://127.0.0.1:8000"),
                voice=tts_cfg.get("voice", "alba"),
                sample_rate=tts_cfg.get("sample_rate", 16000),
            )
        else:
            self.stt = None
            self.llm = None
            self.tts = None

    def _load_config(self, path: str) -> dict:
        p = Path(path)
        if p.exists():
            with open(p, "r", encoding="utf-8") as f:
                return yaml.safe_load(f) or {}
        return {}

    async def handle_connection(self, websocket):
        path = getattr(websocket, "path", None) or getattr(getattr(websocket, "request", None), "path", "/xiaozhi/v1/")
        if not path.startswith(self.endpoint_path.rstrip("/")):
            logger.warning(f"Connection rejected for path: {path}")
            await websocket.close(1008, "Invalid endpoint path")
            return

        session = LocalBrainSession(
            ws=websocket,
            mode=self.mode,
            stt=self.stt,
            llm=self.llm,
            tts=self.tts,
            protocol_log_path=self.protocol_log,
            latency_log_path=self.latency_log,
        )

        logger.info(f"New client connected: {websocket.remote_address}")
        try:
            async for message in websocket:
                if isinstance(message, str):
                    await session.handle_text_frame(message)
                elif isinstance(message, bytes):
                    await session.handle_binary_frame(message)
        except websockets.ConnectionClosed:
            logger.info(f"Client disconnected: {websocket.remote_address}")
        finally:
            await session.cancel_active_turn()

    async def start(self):
        logger.info(f"Starting LocalBrain Hub on ws://{self.host}:{self.port}{self.endpoint_path} (mode={self.mode})")
        async with websockets.serve(self.handle_connection, self.host, self.port):
            await asyncio.Future()  # run forever


def main():
    parser = argparse.ArgumentParser(description="LocalBrain Hub Server")
    parser.add_argument("--config", default="hub/config.yaml", help="Path to config.yaml")
    parser.add_argument("--host", default=None, help="Host to bind to")
    parser.add_argument("--port", type=int, default=None, help="Port to bind to")
    parser.add_argument("--mode", choices=["echo", "full"], default=None, help="Server operation mode")
    args = parser.parse_args()

    hub = LocalBrainHub(config_path=args.config, mode_override=args.mode)
    if args.host:
        hub.host = args.host
    if args.port:
        hub.port = args.port

    try:
        asyncio.run(hub.start())
    except KeyboardInterrupt:
        logger.info("LocalBrain Hub stopped.")


if __name__ == "__main__":
    main()
