from typing import List, Literal

__version__: str

class Finding:
    code: str
    """Ex.: "content.prompt_injection", "content.personal_data", "content.invisible_unicode"."""
    detector: str
    start: int
    """Índice de caractere: texto[start:end] é o trecho."""
    end: int

class Inspection:
    action: Literal["allow", "mask", "block"]
    findings: List[Finding]
    redacted: str
    """O texto com dado pessoal trocado por "[redacted]"."""

def inspect(text: str) -> Inspection: ...
def redact(text: str) -> str: ...
