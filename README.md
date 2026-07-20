# teshi.org

The static GPUI/WebAssembly landing page for [teshi](https://github.com/teshi-org/teshi).

## Requirements

- Rust nightly with the `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` 0.2.126
- Node.js 22 or later

Install the project dependencies with:

```bash
make install
```

## Local development

```bash
make dev
```

Then open <http://127.0.0.1:3000>.

## Production build

```bash
make build
```

The deployable static site is generated in `dist/`.
