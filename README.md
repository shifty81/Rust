# Atlas Engine — Rust Voxel Planet Engine & Editor

A fully procedural **voxel planet engine** written in Rust using [Bevy](https://bevyengine.org/) 0.14,
built around the **Atlas Editor** — a full egui-based editor with Play-In-Editor (PIE), world settings,
solar-system navigation, voxel sculpting tools, and a live outliner.

The runtime voxel game is the primary *example scene* shipped inside the editor.

---

## 📦 Two Entry Points

| Mode | Command | What you get |
|------|---------|--------------|
| **Atlas Editor** | `cargo run -p atlas_editor_app` | Full editor window, PIE, World Settings, solar-system fly-through, voxel tools |
| **Standalone runtime** | `cargo run -p atlas_runtime_app` | First-person voxel exploration, gravity, sprint, jump — no editor UI |

---

## 🌍 Planet

- **1/128th Earth scale** — radius ≈ 49 773 m; curvature imperceptible while walking
- **Procedural terrain** — multi-octave FBM Perlin noise projected onto a sphere
- **Planet overview mesh** — colour-per-vertex UV sphere (180 × 360 segments, ~65 k vertices) visible from orbit
- **Chunk-based voxel terrain** — 16³ voxel chunks generated asynchronously; nearest chunks prioritised; only terrain within render-distance is kept in memory
- **Vertex ambient occlusion** — per-vertex AO baked at mesh time for natural-looking crevice shading with no extra draw calls
- **Ocean sphere** — semi-transparent blue sphere at sea level visible from low orbit

### 🌱 Biomes (12 types)

| Biome | Surface voxels |
|-------|----------------|
| Deep Ocean | Gravel / Sand / Stone |
| Shallow Ocean | Sand / Stone |
| Beach | Sand / Gravel |
| Plains | Grass / Dirt |
| Forest | Grass / Dirt |
| Tropical Forest | Grass / Dirt |
| Desert | Sand / Sandstone |
| Savanna | Grass / Dirt |
| Tundra | Dirt / Stone |
| Arctic | Snow / Ice / Stone |
| Mountain | Rock / Stone |
| Snow Peak | Snow / Rock |

---

## ☀️ Solar System

- **Sun** — emissive G-type star orbiting the planet on an axially-tilted path (10-minute days)
- **Moon** — tidally-influenced orbit (~15-minute period)
- **7 other planets** — Mercury through Neptune analogs, each with its own colour, orbital radius, and period
- **Axial tilt** ≈ 23.5° — provides seasonal variation in sunlight angle
- **Dynamic sky** — midnight navy → twilight orange → daytime blue gradient; sun intensity & tint follow the orbit
- **200 background stars** — colour-coded by stellar type (blue / white / yellow-orange / red)

---

## 🕹️ Character / Player

- **First-person controller** — WASD + mouse-look
- **Spherical gravity** — always pulls toward the planet centre
- **Surface orientation** — the player's local "up" tracks the planet normal anywhere on the globe
- Jump (Space) and sprint (Shift)
- **Flight mode** — press **F** to toggle gravity-free 6DoF flight; Shift boosts to 4 000 m/s
- **PIE HUD** — position, altitude, speed, flight mode, and controls overlay during Play-In-Editor

---

## 🌤 Atmosphere & Weather

- **Dynamic sky colour** — blended from midnight → dawn → full daylight each day cycle
- **Altitude-aware atmosphere** — sky and fog blend to black above 8 km; stars visible in space
- **Weather system** — Clear, Cloudy, Rain, Snow, Storm; transitions every ~90 s
- **Precipitation particles** — rain/snow/storm fall toward the planet centre relative to the player

---

## 🌲 Vegetation

Procedurally placed around the camera, despawned when out of range:

| Species | Biome |
|---------|-------|
| Broadleaf tree | Forest / Tropical Forest |
| Oak tree | Plains / Savanna |
| Pine tree | Tundra |
| Cactus (with arms) | Desert |
| Grass blades | Plains / Forest / Tropical Forest / Savanna |

---

## 🖥️ Atlas Editor

### Editor Window (Editing mode)

| Panel | Description |
|-------|-------------|
| **Viewport** | 3-D free-fly camera over the voxel world; gizmos, snapping, grid |
| **World Outliner** | Hierarchical list of all scene entities grouped by type; multi-select, context menu |
| **Details** | Component inspector for the selected entity; undo-aware transform drag |
| **World Settings** | Live-edit terrain seed, render distance, day/night, weather |
| **Voxel Tools** | Palette, brush mode (Place / Remove / Paint), box / sphere brush, undo-able strokes |
| **Content Browser** | Asset browser panel |
| **Output Log** | Runtime Bevy log messages forwarded to the editor |

### Editor Camera Controls

| Input | Action |
|-------|--------|
| **RMB + drag** | Look around |
| **RMB + WASD** | Fly forward / strafe |
| **RMB + Q / E** | Fly down / up |
| **Scroll wheel** | Multiply speed ×1.25 per notch (1 m/s – 1 000 000 m/s) |
| **Home** | Teleport to solar-system overview |
| **End** | Teleport to planet-surface overview (3 km up) |
| **W / E / R** | Switch gizmo: Translate / Rotate / Scale |
| **G** | Toggle world-grid overlay |
| **Ctrl+Z / Ctrl+Y** | Undo / Redo |
| **Delete** | Delete selected entity |
| **Ctrl+D** | Duplicate selected entity |

### Play-In-Editor (PIE)

Press **▶ Play** in the menu bar to enter PIE:

1. The voxel player spawns above the north pole.
2. The player camera takes over; the editor camera deactivates.
3. A **PIE HUD** overlays position, altitude, speed, and flight mode.
4. Press **⏹ Stop** to return to Editing mode.

### Scene & World I/O

- **File → Save / Open / New** — RON-serialised `.atlasscene` format
- **File → Save / Load World Data** — binary `.voxelworld` format (manually-edited chunks only)
- **Dirty indicator** — `●` in title bar when unsaved changes exist

---

## 🔧 Building & Running

**Requirements:** Rust stable (≥ 1.76 recommended), Linux: `sudo apt install libasound2-dev libudev-dev`.

```bash
# Clone
git clone https://github.com/shifty81/Rust atlas-engine
cd atlas-engine

# Atlas Editor (recommended — runs the full engine inside the editor)
cargo run -p atlas_editor_app

# Atlas Editor — optimised release build
cargo run -p atlas_editor_app --release

# Standalone runtime only (no editor)
cargo run -p atlas_runtime_app --release
```

> **First build:** downloads ~500 Bevy crates — takes several minutes.
> Subsequent incremental builds are fast (seconds).

---

## 🗂 Architecture

```
crates/
├── atlas_editor_app/           — Editor executable (main entry point)
├── atlas_runtime_app/          — Standalone runtime executable
│
├── atlas_voxel_planet/         — Core voxel planet engine
│   ├── src/planet.rs           —   Async chunk generation, AO mesher, noise cache
│   ├── src/biome.rs            —   Biome classification, voxel palette, surface colours
│   ├── src/solar_system.rs     —   Sun, moon, planets, orbital mechanics
│   ├── src/player.rs           —   First-person controller, spherical gravity
│   ├── src/atmosphere.rs       —   Day/night sky, weather, precipitation
│   ├── src/vegetation.rs       —   Procedural tree / grass spawning
│   ├── src/world_io.rs         —   Binary .voxelworld save / load
│   └── src/config.rs           —   All tunable constants
│
├── atlas_editor_core/          — EditorMode state machine, EditorCamera marker
├── atlas_editor_ui/            — Menu bar, snap toolbar, keyboard shortcuts
├── atlas_editor_viewport/      — Free-fly editor camera, viewport picking
├── atlas_editor_play/          — PIE lifecycle + in-game HUD overlay
├── atlas_editor_outliner/      — World Outliner panel
├── atlas_editor_details/       — Details / component inspector panel
├── atlas_editor_world_settings/— World Settings floating panel
├── atlas_editor_scene/         — New / Open / Save .atlasscene events
├── atlas_editor_voxel_tools/   — Voxel palette, brush, DDA ray-cast editing
├── atlas_editor_content/       — Content Browser panel
├── atlas_editor_log/           — Output Log panel (Bevy → egui bridge)
├── atlas_editor_project/       — Project panel
│
├── atlas_commands/             — Undo / Redo command history
├── atlas_gizmos/               — Translate / Rotate / Scale gizmos + grid + snap
├── atlas_selection/            — FocusedEntity resource + SelectionChanged event
├── atlas_scene/                — Scene file format + dirty tracking
├── atlas_prefab/               — Prefab file format + instance overrides
├── atlas_assets/               — Asset registry and metadata
├── atlas_core/                 — StableId, TransformData, Tag (no Bevy dep)
├── atlas_game/                 — Gameplay systems (Player, Health, GamePlugin)
└── atlas_render/               — Render setup helpers

project/                        — Editor project directory (scenes, prefabs, content)
├── Scenes/
├── Prefabs/
├── Content/
├── Config/
└── Cache/
```

---

## 🛣 Planned / Future Work

- Wildlife AI and ecosystem simulation
- Water physics (rivers, oceans with waves)
- Caves and underground biomes
- Space flight / inter-planetary travel
- Inventory and building system
- Multiplayer
- Third-person character model with animations
- Procedural city / structure generation
- Greedy meshing to further reduce chunk triangle count
