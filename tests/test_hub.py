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


@pytest.mark.asyncio
async def test_tts_stream_speech_opus_frames():
    tts_prod = TTSEngine(server_url="http://127.0.0.1:9999", dev_fallbacks=False)
    prod_stream = [f async for f in tts_prod.stream_speech_opus_frames("Streaming test")]
    assert prod_stream == []
    await tts_prod.close()

    tts_dev = TTSEngine(server_url="http://127.0.0.1:9999", dev_fallbacks=True)
    dev_stream = [f async for f in tts_dev.stream_speech_opus_frames("Streaming test")]
    assert len(dev_stream) > 0
    await tts_dev.close()


class MockWebSocket:
    def __init__(self):
        self.sent_messages = []
        self.closed = False
        self.close_code = None
        self.close_reason = None
        self.state = 1

    async def send(self, message):
        self.sent_messages.append(message)

    async def close(self, code=1000, reason=""):
        self.closed = True
        self.close_code = code
        self.close_reason = reason
        self.state = 3


@pytest.mark.asyncio
async def test_session_protocol_frames(tmp_path):
    import json
    from hub.server import LocalBrainSession
    from hub.stt import STTEngine
    from hub.llm import LLMEngine

    ws = MockWebSocket()
    stt = STTEngine(model_size="tiny.en")
    llm = LLMEngine()
    tts = TTSEngine(dev_fallbacks=True)

    session = LocalBrainSession(
        ws=ws,
        mode="full",
        stt=stt,
        llm=llm,
        tts=tts,
        protocol_log_path=tmp_path / "protocol.log",
        latency_log_path=tmp_path / "latency.log",
    )

    # 1. Hello handshake
    await session.handle_text_frame(json.dumps({"type": "hello", "version": 2}))
    assert len(ws.sent_messages) == 1
    hello_resp = json.loads(ws.sent_messages[0])
    assert hello_resp["type"] == "hello"
    assert hello_resp["version"] == 2
    assert hello_resp["features"]["mcp"] is False
    assert hello_resp["session_id"] == session.session_id

    # 2. OTA check
    await session.handle_text_frame(json.dumps({"type": "ota"}))
    assert len(ws.sent_messages) == 2
    ota_resp = json.loads(ws.sent_messages[1])
    assert ota_resp["type"] == "ota"
    assert ota_resp["status"] == "up_to_date"

    # 3. Goodbye
    await session.handle_text_frame(json.dumps({"type": "goodbye"}))
    assert ws.closed is True
    assert ws.close_code == 1000
    assert ws.close_reason == "Client goodbye"

    await tts.close()

