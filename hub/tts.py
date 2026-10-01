import asyncio
import logging
import httpx
import numpy as np

from hub.audio import OpusCodec, wav_to_pcm_bytes


logger = logging.getLogger("localbrain.tts")


class TTSEngine:
    """Async client for pocket-tts serve with Opus frame generation."""

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

    async def generate_speech_opus_frames(
        self,
        text: str,
        voice: str | None = None,
    ) -> list[bytes]:
        """Synthesize text and return a list of 60ms Opus frames."""
        if not text.strip():
            return []

        selected_voice = voice or self.voice
        pcm_bytes = await self._fetch_synthesis_pcm(text, selected_voice)
        if not pcm_bytes:
            return []
        return self.codec.pcm_to_opus_frames(pcm_bytes)

    async def _fetch_synthesis_pcm(self, text: str, voice: str) -> bytes:
        """Call pocket-tts serve HTTP endpoint."""
        url = f"{self.server_url}/tts"
        try:
            resp = await self.client.post(
                url,
                data={"text": text, "voice_url": voice},
            )
            if resp.status_code == 200 and len(resp.content) > 44:
                return wav_to_pcm_bytes(resp.content, target_sample_rate=self.sample_rate)
            else:
                logger.warning(f"Pocket-TTS returned status {resp.status_code}")
        except Exception as e:
            logger.debug(f"Pocket-TTS serve unreachable at {url}: {e}")

        if self.dev_fallbacks:
            return self._generate_fallback_pcm(text)
        return b""

    def _generate_fallback_pcm(self, text: str) -> bytes:
        """Controlled test fallback tone generator."""
        duration = max(0.3, min(1.5, len(text) * 0.05))
        num_samples = int(self.sample_rate * duration)
        t = np.linspace(0, duration, num_samples, endpoint=False)
        tone = np.sin(2 * np.pi * 300 * t) * 0.3
        samples = (tone * 32767).astype(np.int16)
        return samples.tobytes()

    async def close(self):
        await self.client.aclose()
