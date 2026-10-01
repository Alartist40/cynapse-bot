"""Ollama streaming LLM client and sentence chunker for LocalBrain Hub."""

import asyncio
from collections import deque
from pathlib import Path
from typing import AsyncGenerator
import ollama


def split_sentences(text: str) -> list[str]:
    """Split text on sentence boundaries."""
    delimiters = [". ", "! ", "? ", ".\n", "!\n", "?\n", "\n\n"]
    sentences = []
    current = ""
    for char in text:
        current += char
        if any(current.endswith(d) for d in delimiters):
            sentences.append(current.strip())
            current = ""
    if current.strip():
        sentences.append(current.strip())
    return sentences


class LLMEngine:
    """Async Ollama streaming client with persona and conversational memory."""

    def __init__(
        self,
        host: str = "http://127.0.0.1:11434",
        model: str = "qwen2.5:3b-instruct",
        system_prompt_path: str = "hub/persona/system.md",
        temperature: float = 0.7,
        max_history: int = 6,
    ):
        self.host = host
        self.model = model
        self.temperature = temperature
        self.max_history = max_history
        self.client = ollama.AsyncClient(host=host)
        self.system_prompt = self._load_system_prompt(system_prompt_path)

    def _load_system_prompt(self, path: str) -> str:
        p = Path(path)
        if p.exists():
            return p.read_text(encoding="utf-8").strip()
        return "You are StackChan, a helpful voice companion. Keep answers under 2 sentences."

    async def stream_reply_sentences(
        self,
        user_text: str,
        history: deque,
    ) -> AsyncGenerator[str, None]:
        """Stream response chunked into spoken sentences."""
        messages = [{"role": "system", "content": self.system_prompt}]
        for item in history:
            messages.append(item)
        messages.append({"role": "user", "content": user_text})

        buffer = ""
        delimiters = [".", "!", "?", "\n"]

        try:
            stream = await self.client.chat(
                model=self.model,
                messages=messages,
                stream=True,
                options={"temperature": self.temperature},
            )

            async for chunk in stream:
                token = chunk.get("message", {}).get("content", "")
                if not token:
                    continue
                buffer += token

                # 1. Punctuation boundary
                has_punct = any(d in buffer for d in delimiters)
                if has_punct:
                    for d in delimiters:
                        if d in buffer:
                            parts = buffer.split(d, 1)
                            sentence = (parts[0] + d).strip()
                            buffer = parts[1] if len(parts) > 1 else ""
                            if sentence:
                                yield sentence
                            break

                # 2. Length fallback: if buffer >= 80 chars without punctuation, split at last space
                elif len(buffer) >= 80:
                    last_space = buffer.rfind(" ")
                    if last_space != -1 and last_space >= 30:
                        chunk_text = buffer[:last_space].strip()
                        buffer = buffer[last_space + 1 :]
                        if chunk_text:
                            yield chunk_text

            # Yield remaining text
            if buffer.strip():
                yield buffer.strip()

        except asyncio.CancelledError:
            raise
