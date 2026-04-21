# Nova-Forge — Atlas Editor

The **Nova-Forge editor** — a full egui-based Bevy 0.14 editor with
Play-In-Editor (PIE), world settings, voxel sculpting tools, solar-system
navigation, and a live outliner.

> ## 🎯 Role of this repository
>
> **This repo is the editor, not a game.** Its job is to author the content
> (scenes, prefabs, voxel worlds, biomes, creatures, recipes, quests, …) that
> the separate **Nova-Forge game** repository loads and runs.
>
> The `atlas_voxel_planet` crate inside this repo is a **test target / demo
> viewport** for exercising editor features (voxel sculpting, PIE, world
> settings, hot-reload).  It is not the shipping game.
>
> The shipping game binary lives in the Nova-Forge game repo and consumes the
> RON / binary assets produced here — so changes authored in this editor
> propagate to the game without rebuilding the 25-minute engine binary.

Launch the editor:

```
cargo run -p atlas_editor_app
```

The editor opens with the voxel world already loaded.  Use the free-fly camera
to inspect the terrain, then press **Play (▶)** in the toolbar to enter PIE and
walk the surface.  Press **Stop (■)** to return to editor mode.

---

## 🌍 Planet

- **1/128th Earth scale** — radius ≈ 49 773 m; curvature imperceptible while walking
- **Procedural terrain** — multi-octave FBM Perlin noise projected onto a sphere
- **Greedy meshing** — adjacent same-type faces merged per 2-D slice; vertex AO baked at mesh time
- **Planet overview mesh** — colour-per-vertex UV sphere (180 × 360 segments, ~65 k vertices) visible from orbit
- **Chunk-based voxel terrain** — 16³ voxel chunks generated asynchronously; nearest chunks prioritised; only terrain within render-distance is kept in memory
- **Vertex ambient occlusion** — per-vertex AO baked at mesh time for natural-looking crevice shading with no extra draw calls
- **Ocean sphere** — animated wave mesh at sea level; sine-displaced UV sphere rebuilt every other frame
- **Caves** — 3-D FBM noise carved at depth ≥ 4 voxels below the surface

### 🌱 Biomes (15 types)

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
| **Underground — Crystal** | Crystal voxel (polar biomes, depth ≥ 25) |
| **Underground — Magma** | Magma voxel (warm biomes, depth ≥ 25) |
| **Underground — Obsidian** | Obsidian voxel (warm biomes, depth ≥ 25) |

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
- **Third-person mode** — press **V** to toggle; procedural character body (torso, head, arms, legs) with limb-swing and idle-breathing animations
- **Spherical gravity** — always pulls toward the planet centre
- **Surface orientation** — the player's local "up" tracks the planet normal anywhere on the globe
- Jump (Space) and sprint (Shift)
- **Survival stats** — Health (0–100) and Stamina (0–100); sprint drains stamina; fall damage on hard landings; both regenerate over time
- **Flight mode** — press **F** to toggle gravity-free 6DoF flight; Shift boosts to 4 000 m/s
- **Ground HUD** — top-left overlay showing time of day ☀, weather ☁, health ❤ bar, and stamina ⚡ bar
- **Space HUD** — top-right overlay shown above 8 km: altitude, speed, and nearest body distance
- **Minimap** — 96 px biome-colour disk (bottom-right); shows player (⚪), creatures (🔴), structures (🟡), north indicator; refreshes every second
- **PIE HUD** — position, altitude, speed, flight mode, and controls overlay during Play-In-Editor

### Runtime Controls

| Input | Action |
|-------|--------|
| **WASD / Arrows** | Move |
| **Mouse** | Look |
| **Shift** | Sprint (drains stamina) |
| **Space** | Jump |
| **F** | Toggle flight mode |
| **V** | Toggle first/third-person camera |
| **G** | Break voxel (adds to hotbar) |
| **B** | Place active hotbar voxel |
| **1–9** | Select hotbar slot |
| **Scroll wheel** | Cycle hotbar slot |
| **C** | Open / close crafting panel |
| **E** | Interact with nearest NPC (open/close dialogue) |
| **Escape** | Release / lock cursor |

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

## 🐾 Wildlife

Up to 20 creatures spawn biome-appropriately within 80 m of the player and despawn when out of range:

| Creature | Biome |
|----------|-------|
| Deer | Forest / Tropical Forest / Plains / Savanna |
| Rabbit | Forest / Tropical Forest / Plains / Savanna |
| Camel | Desert |
| Polar Bear | Arctic / Tundra |

Creatures wander the surface using a simple AI: random heading changes, surface-following movement along the planet normal.

---

## 🏗️ Inventory, Building & Crafting

- **9-slot hotbar** — cycle with 1–9 keys or mouse wheel; holds any breakable voxel type
- **G** — mine/break the voxel the player is looking at (ray march up to 6 m)
- **B** — place the active-slot voxel type at the targeted face
- **Hotbar HUD** — bottom-screen row of slot boxes showing voxel type and count
- **C** — open / close the crafting panel

### ⚒ Crafting Recipes

| Recipe | Ingredients | Output |
|--------|------------|--------|
| Compressed Stone | 3× Gravel | 2× Stone |
| Sandstone Slab | 3× Sand | 2× Sandstone |
| Rich Soil | 2× Dirt | 1× Grass |
| Crystal Refinement | 3× Obsidian | 1× Crystal |
| Stone Bricks | 2× Stone + 1× Gravel | 3× Sandstone |
| Snow Pack | 2× Snow + 1× Crystal | 2× Ice |

---

## 🏰 Procedural Structures

Up to 8 structures spawn biome-appropriately around the player and despawn when out of range:

| Structure | Biome |
|-----------|-------|
| Hut | Forest / Plains |
| Sandstone Hut | Desert |
| Watch Tower | Savanna / Mountain |
| Ice Hut | Arctic / Snow Peak |
| Ruin | Any land biome |

---

## 🗣️ NPC Dialogue & Quest System

Up to 6 biome-appropriate NPCs spawn near the player.  Walk within 4 m and press **E** to talk.

| NPC | Biomes | Quest |
|-----|--------|-------|
| Elara (Trader) | Plains / Forest | Gravel Run: deliver 5× Gravel → 4× Sandstone |
| Borin (Hermit) | Tundra / Mountain | Stone Supply: deliver 6× Stone → 2× Crystal |
| Zara (Nomad) | Desert / Savanna | Sand Dunes: deliver 8× Sand → 2× Obsidian |
| Cael (Fisherman) | Beach / ShallowOcean | Snowball Supply: deliver 4× Snow → 6× Gravel |
| Vira (Explorer) | Arctic / SnowPeak | Crystal Cache: deliver 3× Crystal → 10× Stone |

- Talking to an NPC **activates** their quest.
- Collect the required items and talk again to **turn in** and receive the reward.
- The dialogue panel shows quest progress (current / required count).

---

## 🎵 Biome-Specific Ambient Audio

A looping ambient track plays based on the player's current biome and cross-fades
(2.5 s fade) when the biome changes.

| Track file (`assets/audio/ambient/`) | Plays in biome |
|--------------------------------------|----------------|
| `plains.ogg` | Plains |
| `forest.ogg` | Forest / Tropical Forest |
| `desert.ogg` | Desert / Savanna |
| `arctic.ogg` | Arctic / Tundra / Snow Peak |
| `mountain.ogg` | Mountain |
| `ocean.ogg` | Beach / Ocean |
| `space.ogg` | Above atmosphere |

Place OGG Vorbis files in `assets/audio/ambient/` to activate audio.
Missing files are handled gracefully — the game runs silently without them.

---

## 🌐 Multiplayer (LAN)

Basic LAN co-op over a non-blocking UDP socket.  No extra dependencies required.

Configure `NetworkConfig` before the app starts:

```rust
app.insert_resource(NetworkConfig {
    role:        NetworkRole::Host,   // or Client / Offline
    port:        7777,
    remote_host: None,                // Some("192.168.x.x") for Client
    player_id:   0,
    ..default()
});
```

- **Host** — binds `0.0.0.0:7777`; relays all player-state packets to every known peer
- **Client** — connects to the host; sends its own state; receives relayed states
- Fixed 32-byte wire format: position, yaw, pitch, speed, is_flying
- Remote players rendered as a simple procedural body; stale peers despawned after 5 s of silence

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

# Nova-Forge editor (recommended — full engine running inside the editor)
cargo run -p atlas_editor_app

# Nova-Forge editor — optimised release build
cargo run -p atlas_editor_app --release
```

> The shipping game binary lives in the separate Nova-Forge game repo and is
> not built from this workspace.

> **First build:** downloads ~500 Bevy crates — takes several minutes.
> Subsequent incremental builds are fast (seconds).

---

## 🗂 Architecture

```
crates/
├── atlas_editor_app/           — Nova-Forge editor executable (main entry point)
│
├── atlas_voxel_planet/         — Test-target voxel engine (demo content for
│                                  exercising editor features; not the shipping
│                                  game — that lives in the Nova-Forge game repo)
│   ├── src/planet.rs           —   Async chunk generation, greedy AO mesher, cave FBM, noise cache
│   ├── src/biome.rs            —   Biome classification, voxel palette, underground biomes
│   ├── src/solar_system.rs     —   Sun, moon, planets, orbital mechanics
│   ├── src/player.rs           —   First-person controller, spherical gravity, health/stamina
│   ├── src/character.rs        —   Third-person body (V key), procedural animations
│   ├── src/atmosphere.rs       —   Day/night sky, weather, precipitation
│   ├── src/vegetation.rs       —   Procedural tree / grass spawning
│   ├── src/wildlife.rs         —   Creature AI spawning (biome-matched, wander)
│   ├── src/inventory.rs        —   Hotbar, voxel break/place (G/B), HUD
│   ├── src/crafting.rs         —   C-key crafting panel, recipe list, ingredient/output transfers
│   ├── src/npc.rs              —   NPC spawn/dialogue (E key), QuestLog with 5 quests
│   ├── src/structures.rs       —   Procedural huts, towers, ruins
│   ├── src/multiplayer.rs      —   LAN UDP host/client, RemotePlayer sync
│   ├── src/hud.rs              —   Ground HUD (time/weather/health/stamina) + Space HUD
│   ├── src/minimap.rs          —   64×64 dynamic biome-colour minimap texture
│   ├── src/ambient_audio.rs    —   Biome-specific looping audio with cross-fade
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

- ~~Wildlife AI and ecosystem simulation~~ ✅ implemented (`wildlife.rs`)
- ~~Water physics (rivers, oceans with waves)~~ ✅ implemented (animated ocean mesh)
- ~~Caves and underground biomes~~ ✅ implemented (FBM cave carving + Crystal/Magma/Obsidian voxels)
- ~~Space flight / inter-planetary travel~~ ✅ implemented (flight mode, space HUD, orbital bodies)
- ~~Inventory and building system~~ ✅ implemented (`inventory.rs` — G/B break/place, hotbar HUD)
- ~~Multiplayer~~ ✅ implemented (`multiplayer.rs` — LAN UDP host/client)
- ~~Third-person character model with animations~~ ✅ implemented (`character.rs` — V key toggle, procedural body)
- ~~Procedural city / structure generation~~ ✅ implemented (`structures.rs` — huts, towers, ruins)
- ~~Greedy meshing to further reduce chunk triangle count~~ ✅ implemented (`planet.rs`)

**Future ideas:**

- ~~Crafting system (recipe-based item combining)~~ ✅ implemented (`crafting.rs`)
- ~~Minimap overlay~~ ✅ implemented (`minimap.rs`)
- ~~NPC dialogue / quest system~~ ✅ implemented (`npc.rs`)
- ~~Biome-specific ambient audio~~ ✅ implemented (`ambient_audio.rs`)
- Dedicated server mode (authoritative host, client-side prediction)
- glTF character model swap (replace procedural body with an animated mesh)
