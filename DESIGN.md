---
name: Project-M Design System
description: Tactical Neon Hi-Fi Audio Link Design System
colors:
  primary: "#00FFCC"
  error: "#FF3366"
  neutral-bg: "#0A0A0E"
  neutral-surface: "#14141B"
  neutral-card: "#1B1B24"
  text-primary: "#FFFFFF"
  text-muted: "#8E8E9F"
  text-disabled: "#555566"
typography:
  display:
    fontFamily: "monospace, Courier New, monospace"
    fontSize: "28px"
    fontWeight: 900
    letterSpacing: "2px"
  body:
    fontFamily: "sans-serif, Arial, Helvetica, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.4
  label:
    fontFamily: "sans-serif, Arial, Helvetica, sans-serif"
    fontSize: "12px"
    fontWeight: 800
    letterSpacing: "0.8px"
rounded:
  sm: "6px"
  md: "8px"
  lg: "12px"
  xl: "14px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "18px"
  xxl: "24px"
components:
  panel:
    backgroundColor: "{colors.neutral-surface}"
    rounded: "{rounded.xl}"
    padding: "{spacing.lg}"
  button-listen:
    backgroundColor: "#00FFCC1E"
    textColor: "{colors.primary}"
    rounded: "{rounded.lg}"
    height: "50px"
  button-stop:
    backgroundColor: "#FF33661E"
    textColor: "#FF4D7D"
    rounded: "{rounded.lg}"
    height: "50px"
---

# Design System: Project-M

## 1. Overview

**Creative North Star: "The Neon Audio Deck"**

Project-M is styled to feel like a premium, tactical, hardware audio receiver. The layout rejects native Windows/Linux system styles in favor of a cohesive, dark, high-contrast, cyberpunk-themed interface. Spacing is dense, information-rich, and organized in rounded cards with clean, glowing neon accents to represent live states.

### Key Characteristics:
* **Dark Interface**: Deep, cool space-black base theme.
* **Status-Driven Color**: Neon cyan acts as the "active/functional" color; hot pink serves as the "stop/error" indicator.
* **Structured Panels**: Unified card groupings with sharp borders and distinct backgrounds instead of nested box elements.
* **Fluid EQ Visualizers**: Real-time multi-color gradients for live audio amplitude feedback.

---

## 2. Colors

The color palette is deliberately restricted, using a dark background palette with high-energy neon status accent highlights.

### Primary
* **Neon Cyan** (`#00FFCC` / `oklch(88% 0.28 174)`): Used for branding, active states, active tab outlines, and successful connection alerts.
* **Neon Pink** (`#FF3366` / `oklch(65% 0.26 355)`): Used for stop actions, disconnected status text, and error indicators.

### Neutral
* **Space Black** (`#0A0A0E`): Main screen background.
* **Dark Violet-Grey** (`#14141B`): Panel and container card backgrounds.
* **Medium Grey** (`#1B1B24`): Input field backdrops and info banner backgrounds.
* **Bright White** (`#FFFFFF`): Primary titles and highlighted values.
* **Cool Grey** (`#B3B3C2`): Body text.
* **Slate Muted** (`#8E8E9F`): Group headers, labels, and secondary instructions.
* **Charcoal Inactive** (`#555566` / `#4C4C5E`): Disabled buttons and footers.

### Named Rules
**The 15% Accent Rule.** Neon accent colors (Cyan / Pink) must never occupy more than 15% of the total screen space. They are reserved exclusively for state outlines, active status indicators, and EQ levels to ensure high visual impact.

---

## 3. Typography

**Display Font:** Monospace (Courier New, system monospace fallback)
**Body/Label Font:** Sans-serif (Inter, Arial, system sans-serif fallback)

### Hierarchy
* **Display (Brand Title)** (Bold 900, `28px`, `2px` letter spacing): Used for the main header brand name `PROJECT-M`.
* **Subtitle (Brand tag)** (Bold 700, `11px`, `1.5px` letter spacing, Uppercase): Used for the tag `HI-FI WIRELESS AUDIO LINK`.
* **Group Headers** (Bold 800, `12px`, `0.8px` letter spacing, Uppercase): Used for panel grouping headers.
* **Input Labels** (Bold 700, `12px`): Used for inline field identifiers (e.g. `Listen Port:`).
* **Body / Messages** (Medium 500, `11px` / `13px`): Used for instruction text and logs.
* **Status Text** (Extra-Bold 900, `14px`): Used for primary status indicator callouts.

---

## 4. Elevation

Project-M conveys depth through **tonal layering** and **colored border outlines** instead of standard diffuse drop shadows.

* **Base Layer**: Deep Space Black (`#0A0A0E`) at rest.
* **Card/Panel Layer**: Rises to `#14141B` with a subtle outline (`1px solid #23232E`).
* **Active Status Layer**: Highlights borders with translucent neon tints (`1px solid #00FFCC4D` for connected states, or `1px solid #FF33664D` for disconnected/error states).
* **Interactive Hover States**: Highlights element backdrops with subtle light changes (`#22222E`) rather than drop shadows.

---

## 5. Components

### Panel Container
* **Background**: `#14141B`
* **Border**: `1px solid #23232E`
* **Border Radius**: `14px` (rounded.xl)
* **Padding**: `16px` (spacing.lg)

### Mode Button (Tab Selector)
* **Background**: Active: `#00FFCC14` (8% opacity); Inactive: `#1B1B24`; Hover: `#22222E`.
* **Border**: Active: `1px solid #00FFCC`; Inactive: `1px solid #2A2A35`; Hover: `1px solid #3D3D4F`.
* **Radius**: `8px` (rounded.md)
* **Height**: `38px`
* **Label**: Active: `#00FFCC` (Bold 800); Inactive: `#B3B3C2`.

### Volume EQ Meter
* **Track Background**: `#1F1F28`
* **Height**: `12px`
* **Radius**: `6px` (rounded.sm)
* **Fill Bar**: Linear gradient from `left` to `right` (`#00FFCC 0%`, `#00FF88 50%`, `#FFCC00 80%`, `#FF3366 100%`).
* **Micro-animation**: Animate width change with `60ms` duration using `ease-out` easing.

### Action Button
* **Background**: Active (Listening): `#FF33661E` (12% opacity); Idle: `#00FFCC1E` (12% opacity).
* **Border**: Active (Listening): `2px solid #FF3366`; Idle: `2px solid #00FFCC`.
* **Height**: `50px`
* **Radius**: `12px` (rounded.lg)
* **Label**: Bold 900, `15px`, letter-spacing `1px`. Active label is `#FF4D7D`; Idle label is `#00FFCC`.

---

## 6. Do's and Don'ts

### Do's
* **Do** use uppercase and wide letter-spacing on display headers and group labels to reinforce the technical/hardware aesthetic.
* **Do** transition borders to translucent green/red (`#00FFCC4C` / `#FF33664C`) on status cards to visually link connection states.
* **Do** keep spacing compact to maximize control readability.
* **Do** style input text fields with matching dark backgrounds and thin grey borders.

### Don'ts
* **Don't** use system default buttons or progress indicators; utilize the custom components detailed above to prevent visual mismatch.
* **Don't** use gradient text for UI controls or headers. Keep text solid white or solid neon.
* **Don't** add arbitrary shadows or blurs. Maintain sharp, high-contrast, flat panels.
* **Don't** allow secondary colors to dominate. The background must remain extremely dark.
