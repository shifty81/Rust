# Biomes (`*.biome.ron`) — **schema stub, not yet implemented**

Per-biome data currently hardcoded in
`crates/atlas_voxel_planet/src/biome.rs`.  Moving this to RON means tuning
biome feel (palette, vegetation density, ambient track) without a rebuild.

## Intended schema

```ron
(
    // Name used by the editor's biome picker.
    name: "Temperate Forest",

    // Ordered voxel stack from the surface down.  Each layer is
    // (voxel-name, thickness-in-voxels).
    surface_stack: [
        ("Grass", 1),
        ("Dirt",  3),
        ("Stone", 0), // 0 = all remaining depth
    ],

    // Per-second spawn weights relative to the default vegetation loop.
    vegetation_weights: (
        tree:  1.0,
        grass: 0.6,
        bush:  0.4,
    ),

    // Which structure kinds the procedural spawner considers in this biome
    // (see ../Structures/).  Empty list ⇒ none.
    structure_kinds: ["Hut", "WatchTower"],

    // Which creatures are allowed here (see ../Creatures/).
    creature_kinds: ["Deer", "Fox"],

    // Filename (relative to `assets/audio/ambient/`) of the ambient loop.
    ambient_track: "forest.ogg",
)
```

## Migration notes
* Split so that pure *selection* (`classify_biome` in `biome.rs`) stays in
  Rust (it's parameterised noise); only the per-biome *tables* move to RON.
* `Biome` enum variants become `StableId` / asset-path refs over time.
