from typing import AsyncGenerator
import asyncio
import logging
import httpx
import numpy as np

from hub.audio import OpusCodec, StreamingWavDecoder, wav_to_pcm_bytes


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
        decoder = StreamingWavDecoder(target_sample_rate=self.sample_rate)

        try:
            async with self.client.stream(
                "POST",
                url,
                data={"text": text, "voice_url": selected_voice},
            ) as response:
                if response.status_code == 200:
                    async for chunk in response.aiter_bytes():
                        for pcm_frame in decoder.feed_chunk(chunk):
                            yield self.codec.encode_frame(pcm_frame)

                    for pcm_frame in decoder.flush():
                        yield self.codec.encode_frame(pcm_frame)
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

