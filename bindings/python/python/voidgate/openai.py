"""One line to guard an OpenAI SDK client (or any client with the same shape:
Ollama, vLLM, Groq, OpenRouter through `base_url`).

    from openai import OpenAI
    import voidgate.openai

    client = voidgate.openai.protect(OpenAI())

Every `client.chat.completions.create(...)`:
  1. scans the messages; blocked -> raises `voidgate.Blocked` before calling the model;
  2. sends the safe copy (personal data replaced);
  3. scans each reply; blocked -> raises `voidgate.Blocked`; new personal data is masked;
  4. with `placeholder="numbered"`, puts the caller's own values back in the reply
     and in the arguments of the tool calls it asks for;
  5. with a `ToolPolicy`, every tool call the model asks for is checked; one that
     needs approval goes to `approve(call)`, and without a yes raises `ToolBlocked`.

Works with `OpenAI` and `AsyncOpenAI`. `stream=True` is not supported yet.
"""

from __future__ import annotations

import inspect as _inspect
import json
from typing import Any, Awaitable, Callable, Union

from ._guard import Blocked, Conversation, Guard, _restore
from ._policy import ToolBlocked, ToolCall, ToolPolicy, _parse

__all__ = ["protect"]

Approver = Callable[[ToolCall], Union[bool, Awaitable[bool]]]


def protect(
    client: Any,
    guard: Guard | None = None,
    *,
    policy: ToolPolicy | None = None,
    approve: Approver | None = None,
) -> Any:
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

    def after(response: Any, conversation: Conversation) -> list[ToolCall]:
        """Checks the reply in place; returns the tool calls that need approval."""
        # Only values the caller sent come back. New data the model made up
        # continues the conversation's numbering ([EMAIL_2]) and stays hidden.
        known = dict(conversation._originals)
        pending: list[ToolCall] = []
        for choice in getattr(response, "choices", ()):
            message = choice.message
            if isinstance(getattr(message, "content", None), str):
                reply = guard._inspect(message.content, conversation._vault)
                if reply.action == "block":
                    raise Blocked(reply)
                message.content = _restore(reply.redacted, known)
            for call in getattr(message, "tool_calls", None) or ():
                function = call.function
                arguments = _parse(function.arguments)
                restored = _restore_arguments(arguments, known)
                # Rewritten only when a placeholder came back, so invalid or
                # untouched arguments reach your code exactly as the model sent them.
                if restored != arguments and isinstance(function.arguments, str):
                    function.arguments = json.dumps(restored, ensure_ascii=False)
                    arguments = restored
                if policy is not None:
                    decision = policy._check(function.name, arguments, conversation)
                    if not decision.allowed:
                        pending.append(decision)
        return pending

    def refuse(call: ToolCall, conversation: Conversation) -> None:
        raise ToolBlocked(call, conversation)

    def guarded(*args: Any, **kwargs: Any) -> Any:
        conversation = before(kwargs)
        result = create(*args, **kwargs)
        # Decided by the result, not the signature: AsyncOpenAI.create sits behind
        # a sync decorator that returns a coroutine.
        if _inspect.isawaitable(result):

            async def finish() -> Any:
                response = await result
                for call in after(response, conversation):
                    verdict = approve(call) if approve else False
                    if _inspect.isawaitable(verdict):
                        verdict = await verdict
                    if verdict is not True:
                        refuse(call, conversation)
                return response

            return finish()
        for call in after(result, conversation):
            verdict = approve(call) if approve else False
            if _inspect.isawaitable(verdict):
                getattr(verdict, "close", lambda: None)()  # no "never awaited" warning
                raise TypeError("voidgate: an async approve needs AsyncOpenAI")
            if verdict is not True:
                refuse(call, conversation)
        return result

    completions.create = guarded
    return client


def _restore_arguments(arguments: dict[str, Any], known: dict[str, str]) -> dict[str, Any]:
    """Puts the caller's values back into each string of the parsed arguments."""

    def walk(value: Any) -> Any:
        if isinstance(value, str):
            return _restore(value, known)
        if isinstance(value, list):
            return [walk(item) for item in value]
        if isinstance(value, dict):
            return {key: walk(item) for key, item in value.items()}
        return value

    restored: dict[str, Any] = walk(arguments)
    return restored
