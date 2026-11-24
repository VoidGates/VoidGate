"""ToolPolicy: what an agent may do after reading untrusted content.

Detection can be fooled; this cannot. The decision depends only on where the
text came from and which tool is called, never on recognizing the attack:

1. a sensitive tool after untrusted content needs approval;
2. a tool not declared in the policy needs approval (deny by default);
3. an argument that copies untrusted text is reported to the approver.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Iterable, Iterator, Mapping

from ._guard import Blocked, Conversation, inspect_messages

#: Roles a message can have in the OpenAI format.
ROLES = frozenset({"system", "developer", "user", "assistant", "tool"})

# Shorter values ("yes", "1") appear everywhere and say nothing about provenance.
_MIN_COPY = 6


@dataclass(frozen=True)
class ToolCall:
    """A tool call the model asked for, and why it needs approval (if it does)."""

    name: str
    arguments: dict[str, Any]
    reasons: list[str]

    @property
    def allowed(self) -> bool:
        """True when no rule applies and the call can run without approval."""
        return not self.reasons


class ToolBlocked(Blocked):
    """A tool call was refused: no approver, or the approver said no."""

    def __init__(self, call: ToolCall, conversation: Conversation | None = None):
        self.call = call
        self.result = conversation  # type: ignore[assignment]
        Exception.__init__(self, f"tool call blocked by VoidGate: {call.name} ({'; '.join(call.reasons)})")


class ToolPolicy:
    """Which tools the agent may call freely and which need approval.

    safe:            read-only tools, always allowed (search, lookups).
    sensitive:       tools with side effects (send, pay, delete); they need
                     approval once untrusted content is in the conversation.
    untrusted_roles: roles whose text may come from an attacker. Default: tool
                     results. Add "user" for a public chatbot.

    Any tool not listed needs approval.
    """

    def __init__(
        self,
        *,
        safe: Iterable[str] = (),
        sensitive: Iterable[str] = (),
        untrusted_roles: Iterable[str] = ("tool",),
    ):
        self.safe = frozenset(safe)
        self.sensitive = frozenset(sensitive)
        self.untrusted_roles = frozenset(untrusted_roles)
        both = self.safe & self.sensitive
        if both:
            raise ValueError(f"tools both safe and sensitive: {sorted(both)}")
        unknown = self.untrusted_roles - ROLES
        if unknown:
            raise ValueError(f"unknown roles in untrusted_roles: {sorted(unknown)}; use {sorted(ROLES)}")
        if not self.untrusted_roles:
            raise ValueError("untrusted_roles cannot be empty")

    def __repr__(self) -> str:
        return (
            f"ToolPolicy(safe={sorted(self.safe)}, sensitive={sorted(self.sensitive)}, "
            f"untrusted_roles={sorted(self.untrusted_roles)})"
        )

    def check(
        self,
        name: str,
        arguments: str | Mapping[str, Any],
        messages: Conversation | Iterable[Mapping[str, Any]],
    ) -> ToolCall:
        """Decides one tool call. `arguments` is the JSON string or dict the model
        produced; `messages` is the conversation that led to it."""
        conversation = messages if isinstance(messages, Conversation) else inspect_messages(messages)
        return self._check(name, _parse(arguments), conversation)

    def _check(self, name: str, arguments: dict[str, Any], conversation: Conversation) -> ToolCall:
        if name in self.safe:
            return ToolCall(name, arguments, [])
        reasons = []
        if name not in self.sensitive:
            reasons.append("tool is not declared in the policy")
        untrusted = [text.casefold() for role, text in conversation._texts if role in self.untrusted_roles]
        # Assistant text may already repeat untrusted content, so it never vouches for a value.
        trusted = [
            text.casefold()
            for role, text in conversation._texts
            if role not in self.untrusted_roles and role != "assistant"
        ]
        if untrusted:
            roles = sorted({role for role, _ in conversation._texts if role in self.untrusted_roles})
            reasons.append(f"conversation contains untrusted content ({', '.join(roles)})")
        for path, value in _strings(arguments):
            needle = value.casefold()
            if (
                len(needle) >= _MIN_COPY
                and any(needle in text for text in untrusted)
                and not any(needle in text for text in trusted)
            ):
                reasons.append(f"argument '{path}' copies untrusted content")
        return ToolCall(name, arguments, reasons)


def _parse(arguments: str | Mapping[str, Any]) -> dict[str, Any]:
    if isinstance(arguments, Mapping):
        return dict(arguments)
    try:
        value = json.loads(arguments)
    except ValueError:
        return {"arguments": arguments}
    return value if isinstance(value, dict) else {"arguments": value}


def _strings(value: Any, path: str = "") -> Iterator[tuple[str, str]]:
    if isinstance(value, str):
        yield path, value
    elif isinstance(value, dict):
        for key, item in value.items():
            yield from _strings(item, f"{path}.{key}" if path else str(key))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from _strings(item, f"{path}[{index}]")
