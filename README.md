# CPMM Fee and Price Impact Calculator

A WebAssembly-based interactive calculator for Constant Product Market Maker (CPMM) mathematics. Built with Rust and designed for embedding in static websites such as Jekyll blogs.

## Features

- Calculate pool reserves from liquidity and price
- Logarithmic price sliders for intuitive adjustment
- Compute wallet deltas for trades between two price points
- Fee calculation on the input side of trades

## Prerequisites

- Rust (edition 2024)
- wasm-pack

Install wasm-pack:

```bash
cargo install wasm-pack
```

## Building

Build the WASM package:

```bash
wasm-pack build --target web
```

This generates the compiled module in the `pkg/` directory.

## Running Locally

WASM modules cannot be loaded from the filesystem due to browser security restrictions. You must serve the files over HTTP.

```bash
python3 -m http.server 8000
```

Then open http://localhost:8000/example.html in your browser.

## Running Tests

```bash
cargo test
```

## Jekyll Integration

1. Copy the `pkg/` directory to your web page assets:

```bash
mkdir -p /path/to/webpage/assets/wasm/post_claude_code_getting_started
cp -r pkg/* /path/to/webpage/assets/wasm/post_claude_code_getting_started/
```

2. Add the following to your web page:

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

3. Style the calculator using the CSS classes in `example.html` as a reference.

## CPMM Mathematics

The calculator uses the constant product invariant:

| Formula | Description |
|---------|-------------|
| k = x · y = L² | Invariant (constant product) |
| P = y / x | Price |
| x = L / √P | Base reserves |
| y = L · √P | Quote reserves |

Where:
- **x**: Base token reserves
- **y**: Quote token reserves
- **L**: Liquidity
- **P**: Spot price

Wallet deltas represent the trader's perspective: positive values indicate tokens received, negative values indicate tokens paid. Fees are collected on the input side of the trade.
