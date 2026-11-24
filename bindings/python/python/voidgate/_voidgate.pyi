__version__: str

def scan(text: str) -> list[tuple[str, str, str, int, int]]:
    """(code, detector, kind, start, end) for every finding; offsets are character indexes."""
