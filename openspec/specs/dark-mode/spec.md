# dark-mode Specification

## Purpose
TBD - created by archiving change add-dark-mode. Update Purpose after archive.
## Requirements
### Requirement: Dark theme palette
The system SHALL provide a dark color palette that overrides the existing CSS custom properties (`--paper`, `--ink`, `--muted`, `--faint`) when the document element has `data-theme="dark"`, producing a dark background with light foreground while preserving the site's warm "paper" mood.

#### Scenario: Dark theme applied
- **WHEN** the `<html>` element has `data-theme="dark"`
- **THEN** the page background uses a dark tone and text/foreground uses a light tone derived from the overridden variables

#### Scenario: Light theme unaffected
- **WHEN** the `<html>` element does NOT have `data-theme="dark"` (default)
- **THEN** the page renders with the original light "paper" palette

### Requirement: Default follows OS preference
The system SHALL, on initial load with no stored manual choice, set the theme to dark when the visitor's OS reports `prefers-color-scheme: dark`, and to light otherwise.

#### Scenario: OS set to dark, no stored choice
- **WHEN** a visitor with `prefers-color-scheme: dark` loads the page for the first time
- **THEN** the page is rendered in dark theme without requiring interaction

#### Scenario: OS set to light, no stored choice
- **WHEN** a visitor with `prefers-color-scheme: light` loads the page for the first time
- **THEN** the page is rendered in light theme

### Requirement: Manual theme toggle
The system SHALL provide a user-activatable control that switches between light and dark themes.

#### Scenario: User toggles to dark
- **WHEN** a visitor activates the theme toggle while in light mode
- **THEN** the theme switches to dark and the control reflects the dark state

#### Scenario: User toggles to light
- **WHEN** a visitor activates the theme toggle while in dark mode
- **THEN** the theme switches to light and the control reflects the light state

### Requirement: Persisted manual choice
The system SHALL store the visitor's manually selected theme in `localStorage` and apply it on subsequent loads, overriding the OS preference.

#### Scenario: Choice persists across reload
- **WHEN** a visitor manually selects dark and then reloads the page
- **THEN** the page renders in dark theme without reverting to the OS preference

#### Scenario: Stored value is invalid
- **WHEN** the stored `localStorage` value is missing or not one of `light`/`dark`
- **THEN** the system ignores it and falls back to the OS preference

### Requirement: No flash of wrong theme
The system SHALL determine and apply the correct theme before first paint so visitors do not see a flash of the wrong theme (FOUC).

#### Scenario: Dark chosen before paint
- **WHEN** the resolved theme (manual or OS) is dark
- **THEN** the dark theme is active at first paint with no visible light flash

### Requirement: Accessible toggle control
The theme toggle SHALL be a keyboard-operable control with an accessible label and state.

#### Scenario: Keyboard and screen-reader accessible
- **WHEN** a visitor uses keyboard or assistive technology to reach the toggle
- **THEN** the control is focusable, activatable, and exposes an accessible name and pressed state

