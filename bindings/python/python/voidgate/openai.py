"""One line to guard an OpenAI SDK client (or any client with the same shape:
Ollama, vLLM, Groq, OpenRouter through `base_url`).

    from openai import OpenAI
    import voidgate.openai

    client = voidgate.openai.protect(OpenAI())

Every `client.chat.completions.create(...)`:
  1. scans the messages; blocked -> raises `voidgate.Blocked` before calling the model;
  2. sends the safe copy (personal data replaced);
  3. scans each reply; blocked -> raises `voidgate.Blocked`; new personal data is masked;
  4. with `placeholder="numbered"`, puts the caller's own values back in the reply.

Works with `OpenAI` and `AsyncOpenAI`. `stream=True` is not supported yet.
"""

from __future__ import annotations

import inspect as _inspect
from typing import Any

from ._guard import Blocked, Conversation, Guard, _restore

__all__ = ["protect"]


def protect(client: Any, guard: Guard | None = None) -> Any:
    """Wraps `client.chat.completions.create` in place and returns the client."""
    guard = guard or Guard(placeholder="numbered")
    completions = client.chat.completions
    create = completions.create

    def before(kwargs: dict[str, Any]) -> Conversation:
        if kwargs.get("stream"):
            raise ValueError("voidgate: stream=True is not supported yet")
        conversation = guard.inspect_messages(kwargs["messages"])
        if conversation.action == "block":
            raise Blocked(conversation)
        kwargs["messages"] = conversation.messages
        return conversation

    def after(response: Any, conversation: Conversation) -> Any:
        for choice in getattr(response, "choices", ()):
            message = choice.message
            if isinstance(getattr(message, "content", None), str):
                # Only values the caller sent come back. New data the model made up
                # continues the conversation's numbering ([EMAIL_2]) and stays hidden.
                known = dict(conversation._originals)
                reply = guard._inspect(message.content, conversation._vault)
                if reply.action == "block":
                    raise Blocked(reply)
                message.content = _restore(reply.redacted, known)
        return response

    def guarded(*args: Any, **kwargs: Any) -> Any:
        conversation = before(kwargs)
        result = create(*args, **kwargs)
        # Decided by the result, not the signature: AsyncOpenAI.create sits behind
        # a sync decorator that returns a coroutine.
        if _inspect.isawaitable(result):

            async def finish() -> Any:
                return after(await result, conversation)

            return finish()
        return after(result, conversation)

    completions.create = guarded
    return client
