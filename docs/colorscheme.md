# WhisperShell Design System & Colorscheme

This document outlines the visual identity and design tokens used throughout the WhisperShell application, ensuring a consistent and premium user experience.

## Colorscheme

Our color palette is designed around a sleek, modern dark mode with vibrant, premium accents.

### Core Backgrounds
- **Primary Background (`--bg-primary`)**: `#000000` (True Black) 
  Used for the main application canvas, providing a deep, immersive canvas that blends seamlessly with dark environments.
- **Card/Container Background (`--bg-card`)**: `#1a1a1a` 
  Used for elevated surfaces like dropdowns, input areas, and the overlay HUD to create visual depth against the true black background.
- **Hover States (`--hover-bg`)**: `rgba(255, 255, 255, 0.05)`
  A subtle, transparent white overlay used for interactive elements like list items and buttons.

### Accents & Indicators
- **Primary Accent (`--lavender`)**: `#b4a0e5`
  The signature color of WhisperShell. Used for primary buttons, active states, progress bar fills, and important icons. Chosen for its premium, calming, yet highly visible aesthetic against dark backgrounds.
- **Danger / Cancellation (`--danger`)**: `#ff8888`
  Used for destructive actions (like canceling a model download) and error states. 

### Borders & Separators
- **Borders (`--border`)**: `rgba(255, 255, 255, 0.1)`
  A low-opacity white used to subtly outline cards and separate list items without drawing focus away from the content.

### Typography Colors
- **Primary Text**: `#ffffff` (Pure White) - Used for primary headings and active text.
- **Secondary Text (`--silver`)**: `#b3b3b3` - Used for standard body copy, labels, and inactive options.
- **Muted Text (`--text-muted`)**: `rgba(255, 255, 255, 0.4)` - Used for disabled items, placeholders, and subtle metadata (like the "Need to be downloaded" tags).

---

## Typography

WhisperShell uses a clean, highly legible font stack powered by Google Fonts, balancing technical precision with modern aesthetics.

### 1. Headings & Titles
- **Font Family**: `Poppins`, sans-serif
- **Usage**: Section titles (e.g., "Model Selection", "Keyboard Shortcut"), overlay status text, and the main application header.
- **Characteristics**: Geometric, friendly, and highly legible. It provides a premium "app-like" feel.

### 2. Standard UI & Body Text
- **Font Family**: `Inter` or `Roboto`, sans-serif (Fallback to system sans-serif)
- **Usage**: Dropdown options, descriptions, standard buttons, and instructional text.
- **Characteristics**: Neutral, highly readable at small sizes (like 11px - 14px), and perfectly suited for dense UI components.

### 3. Technical & Monospace Data
- **Font Family**: `Roboto Mono`, `Fira Code`, or `Courier New`, monospace
- **Usage**: Hotkey badges (e.g., `Ctrl` + `Space`), log messages, and file paths.
- **Characteristics**: Ensures exact character width and alignment, immediately signaling to the user that the information is technical or actionable data.
