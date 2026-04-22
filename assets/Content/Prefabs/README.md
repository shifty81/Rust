# Prefabs (`*.prefab.ron`) — **schema stub, not yet implemented**

Reusable, placeable templates consisting of a **local-space voxel grid**
plus an optional list of child entities.  Replaces the hardcoded
`build_hut`, `watch_tower`, `ice_hut`, `ruin` helpers in
`crates/atlas_voxel_planet/src/structures.rs` so new buildings can be
authored in the editor instead of in Rust.

## Intended schema

```ron
(
    name: "Hut",

    // Extents of the local voxel grid, in voxel units.
    size: (x: 5, y: 4, z: 5),

    // Sparse voxel list.  Any slot not mentioned is air.
    voxels: [
        (at: (0, 0, 0), voxel: "Stone"),
        (at: (4, 0, 0), voxel: "Stone"),
        // …
    ],

    // Optional child entities spawned alongside the voxels — NPCs,
    // props, light sources, lootables, …
    entities: [
        (
            name:        "Campfire",
            transform:   (translation: (2.0, 1.0, 2.0), rotation: (0, 0, 0, 1), scale: (1, 1, 1)),
            components:  [ /* component refs, future */ ],
        ),
    ],
)
```

## Migration notes
* The editor's existing `atlas_editor_voxel_tools` brush will gain a
  "save selection as prefab" action that writes this file.
* `atlas_prefab` already owns the Rust format; this content type is
  mostly the loader + instancer, not new format design.
