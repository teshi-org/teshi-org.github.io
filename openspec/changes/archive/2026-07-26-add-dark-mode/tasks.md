## 1. Dark palette (CSS)

- [x] 1.1 Add a `:root[data-theme="dark"]` block in `assets/css/main.css` that overrides `--paper`, `--ink`, `--muted`, and `--faint` with a warm dark palette.
- [x] 1.2 Adjust the body's SVG noise background (lower opacity / darker tint) under dark theme so the grain reads correctly on the dark base.
- [x] 1.3 (Optional, no-JS fallback) Add a `@media (prefers-color-scheme: dark)` block that applies the dark palette when no `data-theme` is set, so OS-dark users still get dark without JavaScript.

## 2. Pre-paint theme resolution (baseof.html)

- [x] 2.1 In `layouts/_default/baseof.html`, add a small inline `<script>` in `<head>` BEFORE the stylesheet `<link>` that: reads `localStorage` key `theme`; if it is `"light"` or `"dark"` use it, otherwise use `matchMedia('(prefers-color-scheme: dark)').matches`; then sets `document.documentElement.setAttribute('data-theme', ...)`.
- [x] 2.2 Ensure the script is synchronous (no `defer`/`async`) and validates the stored value against the allowed set to avoid invalid states.

## 3. Theme toggle control (baseof.html + CSS)

- [x] 3.1 Add a fixed-position `<button class="theme-toggle">` inside `<body>` in `baseof.html` with `aria-label="Toggle dark mode"`, `aria-pressed`, and a sun/moon glyph reflecting the current theme.
- [x] 3.2 Wire the button's click handler (inline or small script) to toggle `data-theme` between `light` and `dark` and persist the new value to `localStorage`; keep `aria-pressed` in sync.
- [x] 3.3 Add `.theme-toggle` styles in `main.css` (positioning, colors using the CSS variables so it adapts to both themes, hover/focus states, smooth color transition).

## 4. Verification

- [x] 4.1 Run `hugo` (or `hugo server`) and confirm the site builds with no errors.
- [x] 4.2 Manually verify: default follows OS; toggle switches and persists across reload; no flash of wrong theme; keyboard/screen-reader accessible; light theme unchanged.
