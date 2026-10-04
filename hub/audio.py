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


class StreamingWavDecoder:
    """Progressively decodes and resamples streaming WAV audio chunks into 60ms PCM frames."""

    def __init__(self, target_sample_rate: int = SAMPLE_RATE):
        import struct

        self.struct = struct
        self.target_sample_rate = target_sample_rate
        self.header_parsed = False
        self.input_sample_rate = target_sample_rate
        self.channels = 1
        self.bits_per_sample = 16
        self.audio_format = 1
        self.raw_header_buffer = bytearray()
        self.raw_pcm_leftover = bytearray()
        self.in_samples_leftover = np.array([], dtype=np.float32)
        self.out_samples_leftover = np.array([], dtype=np.int16)
        self.frame_samples = int(target_sample_rate * (FRAME_DURATION_MS / 1000.0))  # 960
        self.phase = 0.0

    def feed_chunk(self, chunk: bytes) -> list[bytes]:
        """Feed a raw byte chunk from the HTTP stream and return any full 60ms PCM frames."""
        if not self.header_parsed:
            self.raw_header_buffer.extend(chunk)
            if len(self.raw_header_buffer) < 12:
                return []
            if bytes(self.raw_header_buffer[:4]) != b"RIFF" or bytes(self.raw_header_buffer[8:12]) != b"WAVE":
                # Not a standard RIFF/WAVE container; treat as raw 16kHz int16 PCM
                self.header_parsed = True
                data_bytes = bytes(self.raw_header_buffer)
                self.raw_header_buffer.clear()
                return self._process_pcm(data_bytes)

            pos = 12
            while pos + 8 <= len(self.raw_header_buffer):
                chunk_id = bytes(self.raw_header_buffer[pos : pos + 4])
                chunk_len = self.struct.unpack("<I", self.raw_header_buffer[pos + 4 : pos + 8])[0]
                if chunk_id == b"fmt ":
                    if pos + 8 + chunk_len > len(self.raw_header_buffer):
                        return []
                    self.audio_format, self.channels, self.input_sample_rate, _, _, self.bits_per_sample = self.struct.unpack(
                        "<HHIIHH", self.raw_header_buffer[pos + 8 : pos + 24]
                    )
                    pos += 8 + chunk_len + (chunk_len % 2)
                elif chunk_id == b"data":
                    self.header_parsed = True
                    data_bytes = bytes(self.raw_header_buffer[pos + 8 :])
                    self.raw_header_buffer.clear()
                    return self._process_pcm(data_bytes)
                else:
                    if pos + 8 + chunk_len > len(self.raw_header_buffer):
                        return []
                    pos += 8 + chunk_len + (chunk_len % 2)
            return []
        else:
            return self._process_pcm(chunk)

    def _process_pcm(self, pcm_bytes: bytes) -> list[bytes]:
        if pcm_bytes:
            self.raw_pcm_leftover.extend(pcm_bytes)
        bytes_per_sample = max(1, self.bits_per_sample // 8)
        frame_align = bytes_per_sample * max(1, self.channels)
        aligned_len = (len(self.raw_pcm_leftover) // frame_align) * frame_align
        if aligned_len == 0:
            return []
        aligned_bytes = bytes(self.raw_pcm_leftover[:aligned_len])
        del self.raw_pcm_leftover[:aligned_len]

        if bytes_per_sample == 2:
            num_samples = len(aligned_bytes) // 2
            raw_samples = np.frombuffer(aligned_bytes[: num_samples * 2], dtype=np.int16).astype(np.float32)
        elif bytes_per_sample == 4 and self.audio_format == 3:
            num_samples = len(aligned_bytes) // 4
            raw_samples = np.frombuffer(aligned_bytes[: num_samples * 4], dtype=np.float32) * 32767.0
        else:
            num_samples = len(aligned_bytes) // 2
            raw_samples = np.frombuffer(aligned_bytes[: num_samples * 2], dtype=np.int16).astype(np.float32)

        if self.channels > 1:
            raw_samples = raw_samples.reshape(-1, self.channels).mean(axis=1)

        if len(self.in_samples_leftover) > 0:
            in_samples = np.concatenate([self.in_samples_leftover, raw_samples])
        else:
            in_samples = raw_samples

        if self.input_sample_rate == self.target_sample_rate:
            out_pcm = np.clip(in_samples, -32768, 32767).astype(np.int16)
            self.in_samples_leftover = np.array([], dtype=np.float32)
        else:
            ratio = float(self.input_sample_rate) / float(self.target_sample_rate)
            num_in = len(in_samples)
            if num_in < 2 or self.phase >= num_in - 1:
                self.in_samples_leftover = in_samples
                return []

            max_out_samples = int(np.floor((num_in - 1 - 1e-7 - self.phase) / ratio)) + 1
            if max_out_samples <= 0:
                self.in_samples_leftover = in_samples
                return []

            t_out = self.phase + np.arange(max_out_samples) * ratio
            idx = np.floor(t_out).astype(int)
            frac = t_out - idx
            idx_next = idx + 1

            out_pcm_float = in_samples[idx] * (1.0 - frac) + in_samples[idx_next] * frac
            out_pcm = np.clip(out_pcm_float, -32768, 32767).astype(np.int16)

            t_last = t_out[-1]
            k_last = int(np.floor(t_last))
            self.phase = float(t_last + ratio - k_last)
            self.in_samples_leftover = in_samples[k_last:]

        if len(self.out_samples_leftover) > 0:
            total_out = np.concatenate([self.out_samples_leftover, out_pcm])
        else:
            total_out = out_pcm

        frames = []
        offset = 0
        while offset + self.frame_samples <= len(total_out):
            frame_slice = total_out[offset : offset + self.frame_samples]
            frames.append(frame_slice.tobytes())
            offset += self.frame_samples
        self.out_samples_leftover = total_out[offset:]
        return frames

    def flush(self) -> list[bytes]:
        """Flush remaining audio samples zero-padded to a complete 60ms frame."""
        frames = []
        if len(self.raw_pcm_leftover) > 0:
            extra = self._process_pcm(b"")
            frames.extend(extra)

        if len(self.in_samples_leftover) >= 2 and self.input_sample_rate != self.target_sample_rate:
            ratio = float(self.input_sample_rate) / float(self.target_sample_rate)
            num_in = len(self.in_samples_leftover)
            if self.phase < num_in - 1:
                max_out = int(np.floor((num_in - 1 - 1e-7 - self.phase) / ratio)) + 1
                if max_out > 0:
                    t_out = self.phase + np.arange(max_out) * ratio
                    idx = np.floor(t_out).astype(int)
                    frac = t_out - idx
                    idx_next = np.minimum(idx + 1, num_in - 1)
                    out_pcm_float = self.in_samples_leftover[idx] * (1.0 - frac) + self.in_samples_leftover[idx_next] * frac
                    out_pcm = np.clip(out_pcm_float, -32768, 32767).astype(np.int16)
                    self.out_samples_leftover = np.concatenate([self.out_samples_leftover, out_pcm])
            self.in_samples_leftover = np.array([], dtype=np.float32)

        offset = 0
        while offset + self.frame_samples <= len(self.out_samples_leftover):
            frame_slice = self.out_samples_leftover[offset : offset + self.frame_samples]
            frames.append(frame_slice.tobytes())
            offset += self.frame_samples
        rem = self.out_samples_leftover[offset:]

        if len(rem) > 0:
            padded = np.zeros(self.frame_samples, dtype=np.int16)
            padded[: len(rem)] = rem
            frames.append(padded.tobytes())
            self.out_samples_leftover = np.array([], dtype=np.int16)
        return frames

