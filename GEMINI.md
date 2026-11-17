# Vectorio

## Project Overview
Vectorio is a desktop application written in Rust designed for creating vector drawings and toolpaths for pen plotters. It features four unified workspaces sharing a single document:
- **Design**: Vector shapes, freehand drawing, single-line (Hershey) typography, layers, and independent ink/pen assignments.
- **Create**: Generative algorithmic art scenes with reproducible seeds and real-time parameters.
- **Volume**: 3D mesh rendering (OBJ/STL), multi-pass volumetric shading, optical color mixing, contour isolines, perspective projection, and multiview layouts.
- **Output**: Plotting simulation, pen sorting/scheduling with ink drying delays, travel optimization, and millimeter-accurate SVG export.

## Tech Stack
- **Language**: Rust (2021 edition), Python 3 (build/packaging helper scripts).
- **GUI & Graphics**: `eframe` / `egui` 0.29 with `wgpu` rendering backend.
- **Native OS Integration**: `muda` 0.17 (menus), `rfd` 0.15 (file dialogs), `objc2` (macOS-specific window life-cycle).
- **Core Libraries**:
  - `usvg` 0.44 (SVG parsing and rendering).
  - `image` 0.25 (Raster image processing for vectorization and photo tracing).
  - `meshopt` 0.6.2 (3D mesh decimation and geometry optimization).
  - `hyphenation` 0.8 (Text formatting).
  - `rayon` 1 (Parallel processing for vectorization and mesh generation).
  - `serde` / `serde_json` 1 (Document serialization and `.vectorio` format).
  - `rust-i18n` 3 & `sys-locale` 0.3 (Bilingual UI: Spanish default, English).

## Project Structure
```
vectorio/
├── Cargo.toml                    # Workspace manifest (core + app)
├── AGENTS.md                     # Project instructions and invariants
├── assets/branding/              # Application logos, icons, and palettes
├── crates/
│   ├── vectorio-core/            # Core library: geometry, fills, fonts, 3D, SVG I/O
│   │   ├── fonts/                # Hershey stroke fonts
│   │   └── src/
│   │       ├── generate/         # Algorithmic art generators
│   │       ├── scene/            # 3D scene & camera representations
│   │       ├── volumetric/       # 3D rasterization, lighting, isolines, meshopt
│   │       └── ...               # geom, doc, fill, trace, vectorize, output
│   └── vectorio-app/             # egui GUI desktop application
│       ├── locales/              # i18n YAML dictionaries (es, en)
│       └── src/
│           ├── app.rs            # Main eframe App state and dispatch loop
│           ├── commands.rs       # Command enum, shortcuts, native muda menu
│           ├── macos.rs          # macOS window handling and lifecycle hooks
│           ├── paths.rs          # User data directory resolution
│           └── ...               # Workspace UI modules (design, create, volume, output)
├── packaging/                    # Packaging assets (macOS .icns, notices)
└── scripts/                      # Build and installation scripts (install-macos.py, build-dmg.py)
```

## Key Commands
- **Check Workspace**: `cargo check --workspace`
- **Run Tests**: `cargo test --workspace --locked`
- **Run Application**: `cargo run --release -p vectorio-app`
- **Clean Build Artifacts**: `cargo clean`
- **macOS Build & Install**: `python3 scripts/install-macos.py`
- **macOS DMG Packaging**: `python3 scripts/build-dmg.py`

## Domain Knowledge
- **Pen Plotters**: Output format is physical SVG scaled in millimeters with stroke geometries suitable for physical pens (no filled raster primitives without hatching/cross-hatching).
- **Layer & Ink Separation**: Drawing objects belong to layers, but strokes are mapped to physical inks (pens) with specific nib widths, colors, and drying characteristics.
- **Single-Line Fonts**: Standard TrueType/OpenType fonts outline glyphs; plotters require single-stroke centerline fonts (Hershey fonts).
- **Optical Mixing**: Multiple hatching passes with alternating pen colors to produce blended perceptual hues without physical ink bleeding.

## Permanent Constraints
- **Language**: Preserve Spanish as the primary/default interface language, alongside English translations (`locales/`).
- **Data Model**: Strictly preserve the separation between objects/layers and physical inks/pens in `.vectorio` documents.
- **No Unrequested Refactoring**: Modify only files and lines directly relevant to the task.

## Current Architecture Notes
- Desktop UI runs on `eframe::App` with `wgpu`.
- Native menu bar integration delegates to `muda` (configured for macOS `NSApplication`).
- Window chrome currently configures `with_fullsize_content_view(true)` and `with_titlebar_shown(false)` targeting macOS traffic-light buttons and window styling.
- User storage (`support_dir`) locates configuration and recovery documents under `~/Library/Application Support/Vectorio`.
