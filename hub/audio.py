"""Audio codec and framing utilities for LocalBrain Hub."""

import io
import wave
import numpy as np
import opuslib

SAMPLE_RATE = 16000
CHANNELS = 1
FRAME_DURATION_MS = 60
SAMPLES_PER_FRAME = int(SAMPLE_RATE * (FRAME_DURATION_MS / 1000.0))  # 960
BYTES_PER_FRAME = SAMPLES_PER_FRAME * 2  # 1920 bytes (int16 mono)


class OpusCodec:
    """Manages Opus encoding and decoding for 16 kHz mono 60 ms frames."""

    def __init__(self, sample_rate: int = SAMPLE_RATE, channels: int = CHANNELS):
        self.sample_rate = sample_rate
        self.channels = channels
        self.samples_per_frame = int(sample_rate * (FRAME_DURATION_MS / 1000.0))
        self.encoder = opuslib.Encoder(sample_rate, channels, opuslib.APPLICATION_VOIP)
        self.decoder = opuslib.Decoder(sample_rate, channels)

    def decode_frame(self, opus_bytes: bytes) -> bytes:
        """Decode a single Opus frame to 16-bit PCM mono bytes."""
        return self.decoder.decode(opus_bytes, self.samples_per_frame)

    def encode_frame(self, pcm_bytes: bytes) -> bytes:
        """Encode 60 ms (1920 bytes) of 16-bit PCM mono bytes into an Opus frame."""
        return self.encoder.encode(pcm_bytes, self.samples_per_frame)

    def pcm_to_opus_frames(self, pcm_data: bytes) -> list[bytes]:
        """Convert an arbitrary length PCM buffer into a list of 60ms Opus frames."""
        frames = []
        offset = 0
        frame_bytes_len = self.samples_per_frame * 2
        while offset + frame_bytes_len <= len(pcm_data):
            chunk = pcm_data[offset : offset + frame_bytes_len]
            frames.append(self.encode_frame(chunk))
            offset += frame_bytes_len

        # Pad trailing partial chunk if present
        remaining = len(pcm_data) - offset
        if remaining > 0:
            padded = pcm_data[offset:] + (b"\x00" * (frame_bytes_len - remaining))
            frames.append(self.encode_frame(padded))

        return frames


def pcm_to_wav_bytes(pcm_data: bytes, sample_rate: int = SAMPLE_RATE, channels: int = CHANNELS) -> bytes:
    """Pack raw 16-bit PCM bytes into a WAV container in memory."""
    buf = io.BytesIO()
    with wave.open(buf, "wb") as wav:
        wav.setnchannels(channels)
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        wav.writeframes(pcm_data)
    return buf.getvalue()


def wav_to_pcm_bytes(wav_bytes: bytes, target_sample_rate: int = SAMPLE_RATE) -> bytes:
    """Extract and resample PCM bytes from WAV data."""
    import soundfile as sf
    import scipy.signal

    buf = io.BytesIO(wav_bytes)
    data, sr = sf.read(buf, dtype="int16")
    if data.ndim > 1:
        data = data.mean(axis=1).astype(np.int16)

    if sr != target_sample_rate:
        num_target_samples = int(len(data) * target_sample_rate / sr)
        resampled = scipy.signal.resample(data.astype(np.float32), num_target_samples)
        data = np.clip(resampled, -32768, 32767).astype(np.int16)

    return data.tobytes()
