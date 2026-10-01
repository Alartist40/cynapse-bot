"""Ollama streaming LLM client and sentence chunker for LocalBrain Hub."""

import asyncio
from collections import deque
from pathlib import Path
from typing import AsyncGenerator
import ollama


def extract_earliest_sentence(buffer: str) -> tuple[str | None, str]:
    """Find the earliest sentence boundary in the buffer.

    Returns (extracted_sentence, remaining_buffer).
    """
    delimiters = [".", "!", "?", "\n"]
    earliest_pos = -1
    matched_delim = ""

    for d in delimiters:
        pos = buffer.find(d)
        if pos != -1:
            if earliest_pos == -1 or pos < earliest_pos:
                earliest_pos = pos
                matched_delim = d

    if earliest_pos != -1:
        # Advance past any contiguous punctuation delimiters (e.g. '...', '!!', '?!', etc.)
        end_pos = earliest_pos + len(matched_delim)
        while end_pos < len(buffer) and buffer[end_pos] in ".!?\n":
            end_pos += 1

        sentence = buffer[:end_pos].strip()
        remaining = buffer[end_pos:].lstrip(" \t\n")

        # Guard against yielding pure punctuation or empty fragments
        if not any(c.isalnum() for c in sentence):
            if remaining:
                return extract_earliest_sentence(remaining)
            return None, ""

        return sentence, remaining

    return None, buffer



def split_sentences(text: str) -> list[str]:
    """Split text on earliest sentence boundaries."""
    sentences = []
    buffer = text
    while buffer:
        sentence, remaining = extract_earliest_sentence(buffer)
        if sentence:
            sentences.append(sentence)
            buffer = remaining
        else:
            if buffer.strip():
                sentences.append(buffer.strip())
            break
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

                # 1. Punctuation boundary (earliest occurrence)
                while True:
                    sentence, remaining = extract_earliest_sentence(buffer)
                    if sentence:
                        buffer = remaining
                        yield sentence
                    else:
                        break

                # 2. Length fallback: if buffer >= 80 chars without punctuation, split at last space
                if len(buffer) >= 80:
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

