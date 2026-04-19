# NovaForge — Rust Voxel Planet Engine & Editor

A fully procedural **voxel planet engine** written in Rust using [Bevy](https://bevyengine.org/) 0.14,
packaged as both a standalone first-person game and a full **NovaForge Editor** with
Play-In-Editor (PIE), world settings, solar-system navigation, and a live outliner.

---

## 📦 Two Modes of Use

| Mode | Entry point | What you get |
|------|-------------|--------------|
| **Standalone game** | `cargo run` (repo root) | First-person voxel exploration, gravity, sprint, jump |
| **NovaForge Editor** | `cargo run` inside `novaforge/` | Full editor window, PIE, World Settings, solar-system fly-through |

---

## 🌍 Planet

- **1/128th Earth scale** — radius ≈ 49 773 m; curvature imperceptible while walking
- **Procedural terrain** — multi-octave FBM Perlin noise projected onto a sphere
- **Planet overview mesh** — colour-per-vertex UV sphere (180 × 360 segments, ~65 k vertices) visible from orbit
- **Chunk-based voxel terrain** — 16³ voxel chunks generated dynamically around the camera; only terrain within render-distance is kept in memory
- **Ocean sphere** — semi-transparent blue sphere at sea level visible from low orbit; gives the planet life from a distance

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
- **7 other planets** — Mercury through Neptune, each with its own colour, orbital radius, and period
- **Axial tilt** ≈ 23.5° — provides seasonal variation in sunlight angle
- **Dynamic sky** — midnight navy → twilight orange → daytime blue gradient; sun intensity & tint follow the orbit
- **200 background stars** — emissive spheres distributed by Fibonacci spiral at ≈105 Mm distance; visible colour-coded by stellar type (blue/white/yellow-orange/red)

---

## 🕹️ Character / Player

- **First-person controller** — WASD + mouse-look
- **Spherical gravity** — always pulls toward the planet centre
- **Surface orientation** — the player's local "up" tracks the planet normal anywhere on the globe
- Jump (Space) and sprint (Shift)
- Escape toggles mouse-cursor capture
- **Flight mode** — press **F** to toggle gravity-free 6DoF flight. Walk the surface, then take off into space to explore the solar system:
  - WASD: fly forward/strafe along look direction
  - Q/E: fly down/up along camera axis
  - Shift: boost to 4 000 m/s (quickly reach the moon / sun)
  - F again: return to walking mode (soft-lands via gravity)
- **PIE HUD** — position, altitude (m or km), speed (m/s or km/s), flight mode, and controls displayed during Play-In-Editor

---

## 🌤 Atmosphere & Weather

- **Dynamic sky colour** — blended from midnight → dawn → full daylight each day cycle
- **Altitude-aware atmosphere** — sky and fog blend to black when above 8 km; fully gone by 20 km; in space the background stars are visible and the sun provides clean hard-edge lighting
- **Directional sunlight** — intensity and warm/cool tint follow the sun's position
- **Weather system** — Clear, Cloudy, Rain, Snow, Storm; transitions every ~90 s; intensity slider
- **Precipitation particles** — rain/snow/storm fall toward the planet centre relative to the player

---

## 🌲 Vegetation

Procedurally placed around the player/camera, despawned when out of range:

| Species | Biome |
|---------|-------|
| Broadleaf tree | Forest / Tropical Forest |
| Oak tree | Plains / Savanna |
| Pine tree | Tundra |
| Cactus (with arms) | Desert |
| Grass blades | Plains / Forest / Tropical Forest / Savanna |

---

## 🖥️ NovaForge Editor

The editor (`novaforge/`) wraps the voxel engine with a full egui-based editing environment.

### Editor Window (Editing mode)

| Panel | Description |
|-------|-------------|
| **Viewport** | 3-D free-fly camera over the voxel world |
| **World Outliner** | Hierarchical list of all scene entities grouped by type |
| **Details** | Component inspector for the selected entity |
| **World Settings** | Live-edit terrain seed, render distance, day/night, weather |
| **Content Browser** | Asset browser (placeholder) |
| **Output Log** | Runtime log messages |

### Editor Camera Controls

| Input | Action |
|-------|--------|
| **RMB + drag** | Look around |
| **RMB + WASD** | Fly forward / strafe |
| **RMB + Q / E** | Fly down / up |
| **Scroll wheel** | Multiply speed ×1.25 per notch (range: 1 m/s – 1 000 000 m/s) |
| **Home** | Teleport to solar-system overview (~12 Gm out) |
| **End** | Teleport to planet-surface overview (~3 km up) |
| **W / E / R** | Switch gizmo to Translate / Rotate / Scale |
| **G** | Toggle world-grid overlay |

### Play-In-Editor (PIE)

Press **▶ Play** in the menu bar (or via **View → ▶ Play**) to enter PIE:

1. The voxel player is spawned above the north pole.
2. The player camera takes over; the editor camera is deactivated.
3. A **PIE HUD** overlays FPS, player position, altitude, speed (m/s or km/s), and flight mode.
4. Press **F** to enter flight mode — explore the solar system freely.
5. Press **⏹ Stop** to return to Editing mode; the player entity is despawned.

| Key | PIE Action |
|-----|-----------|
| WASD / Arrows | Walk / fly forward, back, strafe |
| Shift | Sprint (walking) / boost speed ×10 (flying) |
| Space | Jump (walking only) |
| F | Toggle flight / walking mode |
| Q / E | Fly down / up (flight mode only) |
| Esc | Release / capture mouse cursor |

### View Menu — Navigation Shortcuts

| Menu item | Keyboard | Effect |
|-----------|----------|--------|
| 🌌 Solar System Overview | `Home` | Fly to a vantage point showing all planets |
| 🌍 Planet Surface Overview | `End` | Fly to 3 km above the planet's north pole |

### World Settings Panel

- **Terrain** — change the noise seed and press *Regenerate World* to rebuild all chunks
- **Chunks** — adjust render distance (1–20 chunks) and max chunks generated per frame (1–32)
- **Day / Night** — scrub the day-fraction slider in real-time; see axial tilt and day-length constants
- **Weather** — switch weather kind and intensity live
- **Vegetation** — read-only spawn probability and radius constants
- **Player** — read-only walk/run/jump speed, gravity, eye height, and fog distances

---

## 🔧 Building & Running

**Requirements:** Rust ≥ 1.65 (tested with 1.94.1), Linux: `libasound2-dev libudev-dev`.

```bash
# ── Standalone game (repo root) ──────────────────────────────────
cargo run          # debug
cargo run --release  # release (recommended for exploration)

# ── NovaForge Editor ────────────────────────────────────────────
cd novaforge
cargo run          # debug editor
cargo run --release  # release editor
```

> **First build:** downloads ~500 Bevy crates — takes several minutes.
> Subsequent incremental builds are fast.

---

## 🗂 Architecture

```
src/                        ← Standalone game
├── main.rs                 — App entry point; adds all plugins
├── config.rs               — Tunable constants (planet size, orbital periods, …)
├── components.rs           — ECS components and resources
├── biome.rs                — Biome classification, voxel selection, surface colours
├── solar_system.rs         — SolarSystemPlugin: sun, moon, planets, orbital mechanics
├── planet.rs               — PlanetPlugin: overview sphere mesh + voxel chunk manager
├── player.rs               — PlayerPlugin: character controller, spherical gravity, camera
├── atmosphere.rs           — AtmospherePlugin: day/night sky, weather, precipitation
└── vegetation.rs           — VegetationPlugin: procedural tree/cactus spawning

novaforge/                  ← NovaForge Editor workspace
└── crates/
    ├── nf_voxel_planet/    — Re-packaged voxel engine (planet, solar system, player, …)
    ├── nf_editor_app/      — Editor executable (main.rs)
    ├── nf_editor_core/     — EditorMode state machine, EditorCamera marker
    ├── nf_editor_ui/       — Menu bar, toolbar, View menu (Home/End shortcuts)
    ├── nf_editor_viewport/ — Free-fly editor camera + viewport HUD
    ├── nf_editor_play/     — PIE lifecycle + in-game HUD overlay
    ├── nf_editor_outliner/ — World Outliner panel
    ├── nf_editor_details/  — Details / component inspector panel
    ├── nf_editor_world_settings/ — World Settings floating panel
    ├── nf_editor_scene/    — New / Open / Save scene events
    ├── nf_editor_content/  — Content Browser panel
    ├── nf_editor_log/      — Output Log panel
    ├── nf_editor_project/  — Project panel
    ├── nf_commands/        — Undo/Redo command history
    ├── nf_gizmos/          — Translate/Rotate/Scale gizmos + grid toggle
    ├── nf_selection/       — FocusedEntity resource + SelectionChanged event
    ├── nf_assets/          — Asset loading helpers
    ├── nf_scene/           — Scene serialisation stubs
    ├── nf_prefab/          — Prefab stubs
    └── nf_render/          — Render helpers
```

---

## 🛣 Planned / Future Work

- Wildlife AI and ecosystem simulation
- Water physics (rivers, oceans with waves)
- Caves and underground biomes
- Space flight / inter-planetary travel (fly from the planet to the sun or other planets)
- Inventory and building system
- Multiplayer
- Third-person character model with animations
- Procedural city / structure generation
