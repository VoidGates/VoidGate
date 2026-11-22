# Decisão pura

Não conhece HTTP, JSON nem configuração de processo.

Os detectores e a decisão de conteúdo moram no crate [`voidgate-core`](../../crates/voidgate-core). `mod.rs` só reexporta, pra o resto do servidor continuar usando `crate::domain::...`.

| Arquivo ou pasta | O que cuida |
| --- | --- |
| `policy.rs` | Um caminho é isca só quando é igual a um caminho de `VOIDGATE_DECOY_PATHS`. |
| `mod.rs` | Reexporta `action` e `detectors` do core. |
