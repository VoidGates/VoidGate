"""VoidGate: bloqueia manipulação de prompt e mascara dado pessoal antes do modelo de IA.

    import voidgate

    r = voidgate.inspect(texto)
    if r.action == "block":
        ...                      # não mande pro modelo
    texto_seguro = r.redacted    # dado pessoal trocado por [redacted]
"""

from ._voidgate import Finding, Inspection, __version__, inspect, redact

__all__ = ["Finding", "Inspection", "__version__", "inspect", "redact"]
