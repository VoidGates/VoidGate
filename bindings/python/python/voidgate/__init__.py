"""VoidGate: blocks prompt injection and masks personal data before it reaches an AI model.

    import voidgate

    r = voidgate.inspect(text)
    if r.action == "block":
        ...                      # do not send it to the model
    safe = r.redacted            # personal data replaced

Configure with `voidgate.Guard(block=..., allow=..., placeholder=...)`.
Guard an OpenAI-style client in one line with `voidgate.openai.protect(client)`.
"""

from ._guard import (
    PII_KINDS,
    Blocked,
    Conversation,
    Finding,
    Guard,
    Inspection,
    inspect,
    inspect_messages,
    redact,
)
from ._policy import ToolBlocked, ToolCall, ToolPolicy
from ._voidgate import __version__

__all__ = [
    "PII_KINDS",
    "Blocked",
    "Conversation",
    "Finding",
    "Guard",
    "Inspection",
    "ToolBlocked",
    "ToolCall",
    "ToolPolicy",
    "__version__",
    "inspect",
    "inspect_messages",
    "redact",
]
