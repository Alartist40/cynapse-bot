"""Lightweight Faster-Whisper HTTP STT server for LocalBrain Hub."""

import argparse
import io
import wave
import numpy as np
from faster_whisper import WhisperModel
from http.server import HTTPServer, BaseHTTPRequestHandler
import json

model: WhisperModel | None = None


class STTHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == "/transcribe" or self.path == "/v1/audio/transcriptions":
            content_length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_length)

            # Look for WAV header or parse raw bytes
            wav_idx = body.find(b"RIFF")
            if wav_idx != -1:
                wav_bytes = body[wav_idx:]
            else:
                wav_bytes = body

            try:
                with io.BytesIO(wav_bytes) as bio:
                    with wave.open(bio, "rb") as wf:
                        sr = wf.getframerate()
                        frames = wf.readframes(wf.getnframes())
                        audio_np = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0
            except Exception:
                # Direct raw 16kHz s16le PCM fallback
                audio_np = np.frombuffer(wav_bytes, dtype=np.int16).astype(np.float32) / 32768.0

            text = ""
            if model is not None and len(audio_np) > 0:
                segments, _ = model.transcribe(audio_np, language="en", beam_size=1)
                text = " ".join([seg.text.strip() for seg in segments]).strip()

            resp = json.dumps({"text": text}).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(resp)))
            self.end_headers()
            self.wfile.write(resp)
        else:
            self.send_response(404)
            self.end_headers()


def main():
    global model
    parser = argparse.ArgumentParser(description="Faster-Whisper STT Server")
    parser.add_argument("--port", type=int, default=8200)
    parser.add_argument("--model", default="small")
    args = parser.parse_args()

    print(f"Loading faster-whisper model '{args.model}'...")
    model = WhisperModel(args.model, device="auto", compute_type="int8")
    server = HTTPServer(("0.0.0.0", args.port), STTHandler)
    print(f"STT Server listening on http://0.0.0.0:{args.port}/transcribe")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nSTT Server stopped.")


if __name__ == "__main__":
    main()
