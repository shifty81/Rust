# Rust Voxel Planet Engine

A fully procedural voxel planet engine written in Rust using [Bevy](https://bevyengine.org/) (0.14).

## Features

### 🌍 Planet
- **1/128th Earth scale** — radius ≈ 49 773 m, large enough that the curvature is imperceptible when walking on the surface
- **Procedural terrain** using multi-octave FBM (Fractional Brownian Motion) Perlin noise projected onto a sphere
- **Full planet overview mesh** — a colour-per-vertex UV sphere with 180 × 360 segments (~65k vertices)
- **Chunk-based voxel terrain** — 16³ voxel chunks generated dynamically around the player; only terrain within render-distance is kept in memory

### 🌱 Biomes (12 types)
Deep Ocean, Shallow Ocean, Beach, Plains, Forest, Tropical Forest, Desert, Savanna, Tundra, Arctic, Mountain, Snow Peak

Each biome determines:
- Surface and sub-surface voxel materials (Grass, Dirt, Sand, Sandstone, Snow, Ice, Rock, Stone, Gravel…)
- Tree species spawned (Broadleaf, Oak, Pine, Cactus)
- Overview mesh vertex colour

### ☀️ Solar System
- **Sun** — G-type star; orbits the planet creating the day/night cycle (10-minute days by default)
- **Moon** — tidally-influenced orbit (~15-minute period)
- **7 other planets** — Mercury through Neptune, each orbiting the sun at scaled distances with their own visual radii and colours
- **Axial tilt** (≈ 23.5°) applied to the sun's orbit plane — provides seasonal variation

### 🕹️ Character / Player
- **First-person controller** with WASD + mouse-look
- **Spherical gravity** — always pulls toward the planet centre
- **Surface orientation** — the player's local "up" continuously tracks the planet normal as they walk over any part of the globe
- Jump (Space) and sprint (Shift)
- Escape to release / recapture the mouse cursor

### 🌤 Atmosphere & Weather
- **Dynamic sky** — colour gradient blends from midnight navy → twilight orange → daytime blue based on the sun's elevation angle
- **Directional sunlight** — intensity and warm/cool tint follow the sun's position
- **Weather system** (Clear, Cloudy, Rain, Snow, Storm) — transitions at configurable intervals; precipitation rendered as particle systems that fall toward the planet centre

### 🌲 Vegetation
- **Broadleaf trees** (forests)
- **Oak trees** (plains / savanna)
- **Pine trees** (tundra)
- **Cacti** (desert — with side arms)
- Procedurally placed around the player using biome classification and noise-driven moisture maps

## Controls

| Key              | Action                         |
|------------------|--------------------------------|
| W / A / S / D    | Move (forward / left / back / right) |
| Mouse            | Look around                    |
| Space            | Jump                           |
| Left Shift       | Sprint                         |
| Escape           | Toggle mouse capture           |

## Building & Running

**Requirements**: Rust 1.80+, a system with OpenGL/Vulkan/Metal support.

```bash
# Debug build (faster compile, slower runtime)
cargo run

# Release build (recommended for exploring)
cargo run --release
```

> **Note**: The first build downloads and compiles all Bevy dependencies (~500 crates) and takes several minutes.  Subsequent incremental builds are much faster.

## Architecture

```
src/
├── main.rs         — App entry point; adds all plugins
├── config.rs       — All tunable constants (planet size, orbital periods, etc.)
├── components.rs   — ECS components and resources
├── biome.rs        — Biome classification, voxel selection, surface colours
├── solar_system.rs — SolarSystemPlugin: sun, moon, other planets, orbital mechanics
├── planet.rs       — PlanetPlugin: overview sphere mesh + voxel chunk manager
├── player.rs       — PlayerPlugin: character controller, spherical gravity, camera
├── atmosphere.rs   — AtmospherePlugin: day/night sky, weather, precipitation
└── vegetation.rs   — VegetationPlugin: procedural tree/cactus spawning
```

## Planned / Future Work

- Wildlife AI
- Water physics (rivers, oceans with waves)
- Caves and underground biomes
- Space flight / inter-planetary travel
- Inventory and building system
- Multiplayer
