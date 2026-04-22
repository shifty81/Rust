# Nova-Forge content library

This directory holds the **data** that the Nova-Forge editor authors and the
game (and editor PIE) consume at runtime.  Everything here is loaded via
Bevy's asset system and participates in hot-reload — edits to any RON file
while the editor is running propagate live without a 25-minute engine
rebuild.

The categories below correspond to the content types in the
data-driven-editor plan.  Recipes are the first migrated (see
`Recipes/`); the remaining directories are seeded with schema stubs that
follow-up PRs will implement.

```
Content/
├── Recipes/      ← 1. ✅  Migrated.  See Recipes/README.md.
├── Biomes/       ← 2.     Schema stub.  Voxel palette + vegetation /
│                          structure / creature weights + ambient track.
├── Voxels/       ← 3.     Schema stub.  Color, is_solid, hardness, drops.
├── Prefabs/      ← 4.     Schema stub.  Local-space voxel grids +
│                          child-entity list.  Replaces `build_hut` etc.
├── Creatures/    ← 5.     Schema stub.  Kind, mesh refs, stats, biome
│                          affinity, AI params.
├── Characters/   ← 6.     Schema stub.  Player / NPC body-part definitions
│                          (later: glTF refs).
├── Quests/       ← 7.     Schema stub.  Quest definitions + reward tables.
├── Structures/   ← 8.     Schema stub.  Ruin templates, city-block
│                          templates (wrap one-or-more prefabs).
└── Planets/      ← 9.     Schema stub.  Per-planet overrides for the solar
                            system (seed, radius, biome weights, …).
```

## Schema authoring convention

Each content type uses a compound extension (e.g. `*.recipe.ron`,
`*.biome.ron`) so Bevy's `RonAssetLoader` dispatches to the right typed
`Asset`.  Filenames inside a category should be `snake_case` and describe
the content, e.g. `compressed_stone.recipe.ron`,
`temperate_forest.biome.ron`.

## Contract with the game repo

The Rust struct definitions that these files deserialize into live in the
[`atlas_assets`](../../crates/atlas_assets/) crate.  The separate
[Nova-Forge game repo](https://github.com/shifty81/Nova-Forge) consumes
the same structs (or compatible mirrors) so a content pack authored here
drops into the shipping game without changes.
