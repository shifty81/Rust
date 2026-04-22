# Planets (`*.planet.ron`) — **schema stub, not yet implemented**

Per-planet overrides for the solar system, currently hardcoded in
`crates/atlas_voxel_planet/src/solar_system.rs` and
`crates/atlas_voxel_planet/src/config.rs`.

This is the top of the data hierarchy: the **WorldDescriptor** the plan
talks about.  A scene (`*.atlasscene`) points at a planet file; the planet
file references biomes / structures / creatures by name.

## Intended schema

```ron
(
    name: "Atlas",

    // Procedural generation.
    seed:   42_069,
    radius: 2500.0,        // metres.  1/128th Earth ≈ 50 km.

    // Orbital parameters.
    orbit: (
        parent:           "Sun",     // name of another planet/body, or "Sun"
        semi_major_axis:  150_000.0,
        eccentricity:     0.017,
        period_seconds:   86400.0,   // in-game day length
    ),

    // Noise / terrain tuning.
    noise: (
        height_octaves:     6,
        height_persistence: 0.5,
        height_lacunarity:  2.0,
        height_scale:       1500.0,
        cave_threshold:     0.35,
    ),

    // Biome weights (mirror the names in ../Biomes/).
    biome_weights: {
        "Plains":      1.0,
        "Forest":      1.0,
        "Desert":      0.4,
        "Tundra":      0.6,
        "Mountain":    0.8,
    },

    // Atmospheric / weather config.
    atmosphere: (
        fade_start: 500.0,
        fade_end:   2500.0,
        weather_mix: (clear: 0.6, cloud: 0.3, storm: 0.1),
    ),
)
```

## Migration notes
* Scenes reference planets by path, enabling multi-planet solar systems
  where each planet has its own content pack.
* An "Apply & Regenerate" button in the World Settings panel hot-swaps
  the loaded planet descriptor and rebuilds procedural chunks while
  preserving `ManuallyEdited` ones.
