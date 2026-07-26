## Why

The site currently ships a single light "paper" theme. Visitors who prefer dark interfaces (or whose OS is set to dark mode) get a bright, high-contrast page that can be uncomfortable, especially at night. Adding a dark mode improves accessibility and comfort and matches the OS-level theme expectation of modern browsers.

## What Changes

- Introduce a dark color palette using the existing CSS custom-property system (`--paper`, `--ink`, `--muted`, `--faint`), so no markup changes are needed beyond adding a theme toggle.
- Default the theme to the visitor's OS preference via `prefers-color-scheme`, with a manual override.
- Add a small, dependency-free theme toggle control (button) in `baseof.html` that switches between light and dark.
- Persist the visitor's manual choice in `localStorage` so it survives reloads and navigation.
- Apply the chosen theme before first paint (inline script in `<head>`) to avoid a flash of the wrong theme (FOUC).

## Capabilities

### New Capabilities
- `dark-mode`: Theme switching between light and dark, defaulting to `prefers-color-scheme`, with a manual toggle and `localStorage` persistence, applied before first paint.

### Modified Capabilities
<!-- No existing specs to modify. -->

## Impact

- `assets/css/main.css`: add a `[data-theme="dark"]` (and optionally `@media (prefers-color-scheme: dark)`) palette overriding the CSS variables; adjust the textured background for dark.
- `layouts/_default/baseof.html`: add an inline pre-paint script to set `data-theme`, and a toggle button in the body.
- No changes to Hugo config, content, or build pipeline. No new dependencies.
