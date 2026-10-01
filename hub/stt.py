"""Faster-Whisper STT engine for LocalBrain Hub."""

import io
import time
import numpy as np
from faster_whisper import WhisperModel


class STTEngine:
    """Wrapper around faster-whisper CTranslate2 model."""

    def __init__(
        self,
        model_size: str = "small",
        device: str = "auto",
        compute_type: str = "int8",
        language: str = "en",
    ):
        self.model_size = model_size
        self.language = language
        self._model = None
        self._device = device
        self._compute_type = compute_type

    def _ensure_model(self):
        if self._model is None:
            self._model = WhisperModel(
                self.model_size,
                device=self._device,
                compute_type=self._compute_type,
            )

    def transcribe_pcm(self, pcm_data: bytes, sample_rate: int = 16000) -> str:
        """Transcribe 16-bit PCM mono bytes."""
        if not pcm_data:
            return ""

        self._ensure_model()
        audio_np = np.frombuffer(pcm_data, dtype=np.int16).astype(np.float32) / 32768.0

        # Run whisper transcription
        segments, _ = self._model.transcribe(
            audio_np,
            language=self.language,
            beam_size=1,
            vad_filter=True,
        )

        texts = [seg.text.strip() for seg in segments]
        return " ".join(texts).strip()
