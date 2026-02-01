# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Summary

Rust WASM CPMM calculator with DOM injection for web embedding.

## Commands

```bash
cargo test                      # Run all tests
cargo test <name>               # Run single test
cargo clippy                    # Lint
wasm-pack build --target web    # Build WASM to pkg/
python3 -m http.server 8000     # Serve locally (required for WASM)
```

## Structure

```
src/lib.rs      # All implementation (single-file library)
example.html    # Demo page with CSS
pkg/            # WASM build output (generated)
```

## Domain Terminology

- **CPMM**: Constant Product Market Maker (x·y=k invariant)
- **L**: Liquidity (L²=k)
- **P**: Price (y/x)
- **Base/Quote**: Token pair (x=base, y=quote)
- **Wallet delta**: Trade impact from trader perspective (positive=received)

## Architecture

- `CpmmState`: Pool state (L,P) → reserves via `x=L/√P`, `y=L·√P`
- `TradeResult`: Computes deltas and fees between two states
- `AppState`: Shared mutable state via `Rc<RefCell<_>>`
- `inject_ui(anchor_id)`: WASM entry point, builds UI before anchor element

## Gotchas

- `Element` → `Node` conversion required for `append_child`; use `as_node(&elem)`
- Event handlers require `Closure::wrap` + `closure.forget()` to prevent drop
- WASM will not load from `file://`; must serve over HTTP
- Slider uses logarithmic scale: `price = center * 10^((slider-0.5)*2*decades)`

## CSS Classes

`cpmm-calculator`, `cpmm-section`, `cpmm-section-header`, `cpmm-row`, `cpmm-field`, `cpmm-slider-row`, `cpmm-slider`
