## Context

The site is a minimal Hugo (static) landing page. All colors are already defined as CSS custom properties on `:root` in `assets/css/main.css` (`--paper`, `--ink`, `--muted`, `--faint`), and the page background uses a subtle SVG noise texture for a "paper" feel. Because styling is centralized in these variables, a dark theme only needs to override those variables — no per-component changes.

The site has no JavaScript, no build-time theming, and no dependencies. The change must stay dependency-free and work for a static GitHub Pages deploy.

## Goals / Non-Goals

**Goals:**
- Provide a visually consistent dark theme that reuses the existing variable system.
- Respect the OS `prefers-color-scheme` by default.
- Let visitors manually toggle and persist that choice across reloads.
- Prevent a flash of the wrong theme on load (no FOUC).
- Keep changes tiny and dependency-free.

**Non-Goals:**
- No theme picker with multiple accent colors or a full color-customization UI.
- No per-section or scheduled (time-based) theming.
- No changes to Hugo config, content, or the build pipeline.

## Decisions

### D1. Theme mechanism: `data-theme` attribute on `<html>` over CSS-only media query
- **Decision:** Apply themes by setting `data-theme="dark"` (or `"light"`) on the `<html>` element. The dark palette is defined as `:root[data-theme="dark"] { ... }` overrides in CSS.
- **Rationale:** A pure `@media (prefers-color-scheme: dark)` rule gives no manual control. The `data-theme` attribute supports both OS default and manual override in one model.
- **Alternatives considered:** CSS-only media query (rejected — no manual toggle); separate stylesheets per theme (rejected — duplicates the variable block and adds a request).

### D2. Default resolution: OS preference, then manual override
- **Decision:** If a manual choice is stored in `localStorage`, use it. Otherwise fall back to `window.matchMedia('(prefers-color-scheme: dark)').matches`.
- **Rationale:** Matches visitor expectation that the site follows their system unless they opt out. Persisted choice wins so the toggle "sticks."

### D3. Pre-paint inline script to avoid FOUC
- **Decision:** Place a small inline `<script>` in `<head>` (inside `baseof.html`, before the stylesheet `<link>`) that synchronously sets `data-theme` on `document.documentElement` before first paint.
- **Rationale:** An external/deferred script would run after CSS paint and cause a visible flash. Inline + synchronous is the standard no-flash pattern.
- **Trade-off:** Inline JS is not fingerprinted/minified like the CSS, but it is tiny (~10 lines) and worth the FOUC avoidance.

### D4. Toggle control
- **Decision:** Add a `<button>` in `baseof.html` (fixed position, like the existing `.gh` GitHub link) with `aria-label`, `aria-pressed`, and a sun/moon glyph. Clicking toggles `data-theme` and writes the new value to `localStorage`.
- **Rationale:** A native `<button>` is keyboard-accessible and screen-reader friendly for free. Reuses the existing fixed-position pattern already used for `.gh`.

### D5. Dark palette and texture
- **Decision:** In dark mode, set `--paper` to a near-black warm tone (e.g. `#1a1714`), `--ink` to a paper-ish light tone (e.g. `#f4efe6`), and adjust `--muted`/`--faint` accordingly. Reuse the same SVG noise but lower its opacity so the grain reads correctly on dark.
- **Rationale:** Inverting the existing warm "paper" mood keeps brand coherence instead of going clinical blue-black.

## Risks / Trade-offs

- **[Risk] FOUC if script is blocked/deferred** → Mitigation: keep the script inline and synchronous in `<head>`, before the CSS link.
- **[Risk] Stale `localStorage` value if the toggle model changes later** → Mitigation: script validates the stored value against the allowed set (`"light"|"dark"`) and ignores anything else.
- **[Risk] Visitors with JS disabled see only the OS default** → Mitigation: acceptable; the inline script is the only JS and degrades gracefully to `prefers-color-scheme` via a CSS `@media` fallback (see tasks).
- **[Trade-off]** Manual override means a visitor who picks light will not auto-follow a later OS switch — expected behavior, toggle clears are not required.

## Migration Plan

- Pure additive change (new CSS rules, one inline script, one button). No existing markup/CSS removed.
- Deploy by normal Hugo build; no config or data migration.
- **Rollback:** remove the inline script, the toggle button, and the `[data-theme="dark"]` CSS block; site returns to single light theme.

## Open Questions

- None blocking. Optional later enhancement: a `prefers-color-scheme` CSS fallback block so no-JS users still get dark when their OS is dark.
