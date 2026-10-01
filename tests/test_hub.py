import pytest
import numpy as np
from hub.audio import OpusCodec, pcm_to_wav_bytes, wav_to_pcm_bytes
from hub.llm import split_sentences
from hub.tts import TTSEngine


def test_sentence_chunking():
    text = "Hello there! How are you doing today? I am fine. This is a test without punctuation"
    sentences = split_sentences(text)
    assert len(sentences) == 4
    assert sentences[0] == "Hello there!"
    assert sentences[1] == "How are you doing today?"
    assert sentences[2] == "I am fine."
    assert sentences[3] == "This is a test without punctuation"


def test_sentence_chunking_earliest_order():
    text = "Hello! How are you."
    sentences = split_sentences(text)
    assert len(sentences) == 2
    assert sentences[0] == "Hello!"
    assert sentences[1] == "How are you."


def test_sentence_chunking_ellipsis():
    text = "Wait... x"
    sentences = split_sentences(text)
    assert len(sentences) == 2
    assert sentences[0] == "Wait..."
    assert sentences[1] == "x"




def test_opus_codec_roundtrip():
    codec = OpusCodec(sample_rate=16000, channels=1)
    # 60ms frame = 960 samples = 1920 bytes
    t = np.linspace(0, 0.06, 960, endpoint=False)
    pcm_sine = (np.sin(2 * np.pi * 440 * t) * 10000).astype(np.int16).tobytes()

    encoded = codec.encode_frame(pcm_sine)
    assert len(encoded) > 0

    decoded = codec.decode_frame(encoded)
    assert len(decoded) == 1920

    # Multi-frame batch
    multi_pcm = pcm_sine * 3
    frames = codec.pcm_to_opus_frames(multi_pcm)
    assert len(frames) == 3


def test_wav_pcm_conversion():
    pcm_in = (b"\x00\x00" * 960)
    wav_bytes = pcm_to_wav_bytes(pcm_in, sample_rate=16000, channels=1)
    assert wav_bytes.startswith(b"RIFF")
    pcm_out = wav_to_pcm_bytes(wav_bytes, target_sample_rate=16000)
    assert len(pcm_out) == len(pcm_in)


@pytest.mark.asyncio
async def test_tts_honest_offline_behavior():
    tts_prod = TTSEngine(server_url="http://127.0.0.1:9999", dev_fallbacks=False)
    frames = await tts_prod.generate_speech_opus_frames("Hello test")
    assert frames == []
    await tts_prod.close()

    tts_dev = TTSEngine(server_url="http://127.0.0.1:9999", dev_fallbacks=True)
    dev_frames = await tts_dev.generate_speech_opus_frames("Hello test")
    assert len(dev_frames) > 0
    await tts_dev.close()
