from typing import AsyncGenerator
import asyncio
import logging
import httpx
import numpy as np

from hub.audio import OpusCodec, wav_to_pcm_bytes


logger = logging.getLogger("localbrain.tts")


class TTSEngine:
    """Async client for pocket-tts serve with progressive Opus frame streaming."""

    def __init__(
        self,
        server_url: str = "http://127.0.0.1:8000",
        voice: str = "alba",
        sample_rate: int = 16000,
        dev_fallbacks: bool = False,
    ):
        self.server_url = server_url.rstrip("/")
        self.voice = voice
        self.sample_rate = sample_rate
        self.dev_fallbacks = dev_fallbacks
        self.codec = OpusCodec(sample_rate=sample_rate)
        self.client = httpx.AsyncClient(timeout=15.0)
        self.frame_bytes_len = int(sample_rate * 0.06) * 2  # 1920 bytes for 60ms @ 16kHz mono 16-bit

    async def stream_speech_opus_frames(
        self,
        text: str,
        voice: str | None = None,
    ) -> AsyncGenerator[bytes, None]:
        """Stream 60ms Opus frames progressively from pocket-tts as audio is generated."""
        if not text.strip():
            return

        selected_voice = voice or self.voice
        url = f"{self.server_url}/tts"
        pcm_buffer = bytearray()
        header_stripped = False

        try:
            async with self.client.stream(
                "POST",
                url,
                data={"text": text, "voice_url": selected_voice},
            ) as response:
                if response.status_code == 200:
                    async for chunk in response.aiter_bytes():
                        if not header_stripped:
                            pcm_buffer.extend(chunk)
                            if len(pcm_buffer) >= 44:
                                # Strip 44-byte WAV header if present (RIFF header)
                                if bytes(pcm_buffer[:4]) == b"RIFF":
                                    pcm_buffer = pcm_buffer[44:]
                                header_stripped = True
                        else:
                            pcm_buffer.extend(chunk)

                        while len(pcm_buffer) >= self.frame_bytes_len:
                            frame_pcm = bytes(pcm_buffer[: self.frame_bytes_len])
                            del pcm_buffer[: self.frame_bytes_len]
                            yield self.codec.encode_frame(frame_pcm)

                    # Flush remaining partial frame if present
                    if pcm_buffer:
                        padded = bytes(pcm_buffer) + (b"\x00" * (self.frame_bytes_len - len(pcm_buffer)))
                        yield self.codec.encode_frame(padded)
                    return
                else:
                    logger.warning(f"Pocket-TTS returned status {response.status_code}")
        except Exception as e:
            logger.debug(f"Pocket-TTS stream unreachable at {url}: {e}")

        if self.dev_fallbacks:
            for frame in self._generate_fallback_frames(text):
                yield frame

    async def generate_speech_opus_frames(
        self,
        text: str,
        voice: str | None = None,
    ) -> list[bytes]:
        """Synthesize text and return a list of 60ms Opus frames."""
        frames = []
        async for frame in self.stream_speech_opus_frames(text, voice):
            frames.append(frame)
        return frames

    def _generate_fallback_frames(self, text: str) -> list[bytes]:
        """Controlled test fallback tone generator."""
        duration = max(0.3, min(1.5, len(text) * 0.05))
        num_samples = int(self.sample_rate * duration)
        t = np.linspace(0, duration, num_samples, endpoint=False)
        tone = np.sin(2 * np.pi * 300 * t) * 0.3
        samples = (tone * 32767).astype(np.int16)
        return self.codec.pcm_to_opus_frames(samples.tobytes())

    async def close(self):
        await self.client.aclose()

