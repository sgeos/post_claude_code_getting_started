# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust WebAssembly project that implements a Constant Product Market Maker (CPMM) fee and price impact calculator. The compiled WASM module injects an interactive UI into web pages, intended for embedding in web pages.

## Build Commands

```bash
# Run tests
cargo test

# Run a single test
cargo test test_cpmm_state_reserves

# Lint with clippy
cargo clippy

# Build WASM package for web deployment
wasm-pack build --target web
```

The WASM build outputs to `pkg/` and requires `wasm-pack` to be installed.

## Local Testing

```bash
python3 -m http.server 8000
# Then open http://localhost:8000/example.html
```

## Architecture

### Core Types (src/lib.rs)

- **CpmmState**: Represents pool state with liquidity (L) and price (P). Computes reserves using `x = L/sqrt(P)` and `y = L*sqrt(P)`.
- **TradeResult**: Computes wallet deltas and fee collection when moving between two CPMM states. Fees are collected on the input side.
- **AppState**: Mutable application state wrapped in `Rc<RefCell<_>>` for sharing across event handlers.

### UI Pattern

The `inject_ui(anchor_id)` function is the WASM entry point. It locates a DOM element by ID and builds the calculator UI before that element. Event listeners use `Closure::wrap` with `closure.forget()` to bridge Rust closures to JavaScript callbacks.

### web-sys Element Handling

When appending DOM elements, use `as_node(&element)` to convert `Element` to `&Node` for `append_child` calls.

## Jekyll Integration

```html
<script type="module" id="cpmm_calculator">
  import init, { inject_ui } from "/assets/wasm/post_claude_code_getting_started/post_claude_code_getting_started.js";
  async function run() {
    await init();
    inject_ui("cpmm_calculator");
  }
  run();
</script>
```

The script's `id` attribute serves as the anchor point for UI injection.

## CSS Classes

The generated UI uses these classes for styling: `cpmm-calculator`, `cpmm-section`, `cpmm-section-header`, `cpmm-row`, `cpmm-field`, `cpmm-slider-row`, `cpmm-slider`.
