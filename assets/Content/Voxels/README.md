# Voxels (`*.voxel.ron`) — **schema stub, not yet implemented**

Per-voxel-type data currently hardcoded in the `Voxel` enum's `color()` /
`is_solid()` match arms in `crates/atlas_voxel_planet/src/biome.rs`.
Moving this out of the enum lets content authors add new block types
without editing Rust source.

## Intended schema

```ron
(
    // Stable numeric id written to `.voxelworld` save files.  Reserved
    // slots 0–13 correspond to the current hardcoded enum.
    id: 14,
    name: "Basalt",

    // SRGB vertex colour [r, g, b, a] used by the greedy mesher.
    color: (0.20, 0.20, 0.22, 1.0),

    // Blocks player movement and light.  `false` = air/water-like.
    is_solid: true,

    // Future: hardness, break-tool requirement, drops.
    hardness: 3.0,
    drops: [
        (voxel: "Stone", count: 1),
    ],
)
```

## Migration notes
* Existing `Voxel` enum stays (it's in the greedy mesher hot path); at
  load time a `VoxelTable` resource is built that maps id → colour /
  flags.  The enum variants become *stable numeric IDs* rather than
  behaviour containers.
* Adding a new voxel type then *does* still require a Rust change (a new
  enum variant reserved for it) but the visuals / drops / hardness are
  authorable in the editor.
