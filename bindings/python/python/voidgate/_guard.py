"""Policy on top of the Rust scan: what blocks, placeholders, conversations."""

from __future__ import annotations

import copy
import json
import re
from dataclasses import dataclass, field
from typing import Any, Callable, Iterable, Literal, Mapping

from ._voidgate import scan as _scan

Action = Literal["allow", "mask", "block"]
Placeholder = Literal["redacted", "kind", "numbered"]

#: Kinds of personal data. Anything else (`prompt_injection`, `invisible_unicode`) blocks.
PII_KINDS = frozenset({"email", "phone", "card", "iban", "cpf", "cnpj", "ssn"})

_RANK = {"allow": 0, "mask": 1, "block": 2}


@dataclass(frozen=True)
class Finding:
    """One suspicious span. `text[start:end]` is the span; the text itself is never kept."""

    code: str
    detector: str
    kind: str
    start: int
    end: int


@dataclass(frozen=True)
class Inspection:
    """What one text contains and what to do with it."""

    action: Action
    findings: list[Finding]
    redacted: str
    _originals: Mapping[str, str] = field(default_factory=dict, repr=False, compare=False)

    def restore(self, text: str) -> str:
        """Puts back the values hidden by `numbered` placeholders (e.g. in the model reply)."""
        return _restore(text, self._originals)


@dataclass(frozen=True)
class Conversation:
    """A whole message list: the worst action, the safe copy to send, one inspection per text."""

    action: Action
    messages: list[dict[str, Any]]
    inspections: list[Inspection]
    _originals: Mapping[str, str] = field(default_factory=dict, repr=False, compare=False)
    _vault: Any = field(default=None, repr=False, compare=False)
    # (role, original text) of every scanned text, for ToolPolicy. Never in repr.
    _texts: tuple[tuple[str, str], ...] = field(default=(), repr=False, compare=False)

    def restore(self, text: str) -> str:
        """Puts back the values hidden by `numbered` placeholders in this conversation."""
        return _restore(text, self._originals)


class Blocked(Exception):
    """Raised by the SDK wrappers when a request or reply is blocked."""

    def __init__(self, result: Inspection | Conversation):
        self.result = result
        kinds = sorted({f.kind for f in _findings(result)} - PII_KINDS) or ["personal_data"]
        super().__init__(f"blocked by VoidGate: {', '.join(kinds)}")


class Guard:
    """A configured scanner. Reuse one instance; it holds no state between calls.

    block:       personal-data kinds that block instead of mask, e.g. `{"card"}`.
    allow:       values never flagged, e.g. your support e-mail (case-insensitive).
    placeholder: `"redacted"` -> `[redacted]`, `"kind"` -> `[EMAIL]`,
                 `"numbered"` -> `[EMAIL_1]`, reversible with `.restore()`.
    """

    def __init__(
        self,
        *,
        block: Iterable[str] = (),
        allow: Iterable[str] = (),
        placeholder: Placeholder = "redacted",
    ):
        self.block = frozenset(block)
        unknown = self.block - PII_KINDS
        if unknown:
            raise ValueError(f"unknown kinds in block: {sorted(unknown)}; use {sorted(PII_KINDS)}")
        if placeholder not in ("redacted", "kind", "numbered"):
            raise ValueError("placeholder must be 'redacted', 'kind' or 'numbered'")
        self.allow = frozenset(value.casefold() for value in allow)
        self.placeholder = placeholder

    def __repr__(self) -> str:
        return f"Guard(block={sorted(self.block)}, allow={len(self.allow)} values, placeholder={self.placeholder!r})"

    def inspect(self, text: str) -> Inspection:
        """Scans one text."""
        return self._inspect(text, _Vault())

    def inspect_messages(self, messages: Iterable[Mapping[str, Any]]) -> Conversation:
        """Scans an OpenAI-style message list: string and text-part content, and
        tool-call arguments. The input is not modified; `messages` is a safe copy.
        Placeholders are shared, so the same value gets the same `[EMAIL_1]`."""
        vault = _Vault()
        safe: list[dict[str, Any]] = []
        inspections: list[Inspection] = []
        texts: list[tuple[str, str]] = []
        role = ""

        def check(text: str) -> str:
            texts.append((role, text))
            result = self._inspect(text, vault)
            inspections.append(result)
            return result.redacted

        for original in messages:
            message: dict[str, Any] = copy.deepcopy(dict(original))
            role = str(message.get("role", ""))
            content = message.get("content")
            if isinstance(content, str):
                message["content"] = check(content)
            elif isinstance(content, list):
                for part in content:
                    if isinstance(part, dict) and isinstance(part.get("text"), str):
                        part["text"] = check(part["text"])
            for call in message.get("tool_calls") or ():
                function = call.get("function") if isinstance(call, dict) else None
                if function and isinstance(function.get("arguments"), str):
                    function["arguments"] = _check_arguments(function["arguments"], check)
            safe.append(message)

        action: Action = max((i.action for i in inspections), key=_RANK.__getitem__, default="allow")
        return Conversation(action, safe, inspections, vault.originals, vault, tuple(texts))

    def _inspect(self, text: str, vault: _Vault) -> Inspection:
        findings = []
        for code, detector, kind, start, end in _scan(text):
            if kind in PII_KINDS and text[start:end].casefold() in self.allow:
                continue
            findings.append(Finding(code, detector, kind, start, end))
        blocking = any(f.kind not in PII_KINDS or f.kind in self.block for f in findings)
        action: Action = "block" if blocking else "mask" if findings else "allow"
        return Inspection(action, findings, self._redact(text, findings, vault), vault.originals)

    def _redact(self, text: str, findings: list[Finding], vault: _Vault) -> str:
        out, cursor = [], 0
        for f in findings:
            if f.kind not in PII_KINDS or f.start < cursor:
                continue
            out.append(text[cursor : f.start])
            out.append(vault.placeholder(self.placeholder, f.kind, text[f.start : f.end]))
            cursor = f.end
        out.append(text[cursor:])
        return "".join(out)


class _Vault:
    """Placeholder numbering for one text or one conversation."""

    def __init__(self) -> None:
        self.originals: dict[str, str] = {}
        self._by_value: dict[tuple[str, str], str] = {}
        self._count: dict[str, int] = {}

    def placeholder(self, style: Placeholder, kind: str, value: str) -> str:
        if style == "redacted":
            return "[redacted]"
        if style == "kind":
            return f"[{kind.upper()}]"
        key = (kind, value)
        if key not in self._by_value:
            self._count[kind] = self._count.get(kind, 0) + 1
            token = f"[{kind.upper()}_{self._count[kind]}]"
            self._by_value[key] = token
            self.originals[token] = value
        return self._by_value[key]


def _check_arguments(arguments: str, check: Callable[[str], str]) -> str:
    """Tool-call arguments are JSON: scan each string value, keep the JSON valid."""
    try:
        data = json.loads(arguments)
    except ValueError:
        return check(arguments)

    def walk(value: Any) -> Any:
        if isinstance(value, str):
            return check(value)
        if isinstance(value, list):
            return [walk(v) for v in value]
        if isinstance(value, dict):
            return {k: walk(v) for k, v in value.items()}
        return value

    return json.dumps(walk(data), ensure_ascii=False)


def _restore(text: str, originals: Mapping[str, str]) -> str:
    """Models rewrite placeholders (`[CPF_1]` -> `CPF_1`, `cpf_1`), so brackets and
    case are optional; `\\b` keeps `CPF_1` from matching inside `CPF_10`."""
    if not originals:
        return text
    by_name = {token.strip("[]").casefold(): value for token, value in originals.items()}
    names = "|".join(re.escape(token.strip("[]")) for token in originals)
    pattern = re.compile(rf"\[?\b({names})\b\]?", re.IGNORECASE)
    return pattern.sub(lambda match: by_name[match.group(1).casefold()], text)


def _findings(result: Inspection | Conversation) -> Iterable[Finding]:
    if isinstance(result, Inspection):
        return result.findings
    return (f for i in result.inspections for f in i.findings)


_default = Guard()


def inspect(text: str) -> Inspection:
    """Scans one text with the default policy."""
    return _default.inspect(text)


def inspect_messages(messages: Iterable[Mapping[str, Any]]) -> Conversation:
    """Scans a message list with the default policy."""
    return _default.inspect_messages(messages)


def redact(text: str) -> str:
    """`text` with personal data replaced by `[redacted]`."""
    return _default.inspect(text).redacted
