# Recipes (`*.recipe.ron`)

Crafting recipes.  **✅ Migrated** — this is the first data-driven content
type in Nova-Forge and the reference for all future migrations.

## Schema

```ron
(
    name: "Compressed Stone",
    ingredients: [
        (voxel: "Gravel", count: 3),
    ],
    output: (voxel: "Stone", count: 2),
)
```

| Field         | Type                 | Notes                                             |
|---------------|----------------------|---------------------------------------------------|
| `name`        | string               | Shown in the crafting panel row.                  |
| `ingredients` | list of ingredient   | All must be satisfied simultaneously to craft.    |
| `output`      | voxel + count        | What the recipe produces.                         |

An ingredient is `(voxel: <name>, count: <u32>)` where `<name>` is one of the
`atlas_voxel_planet::biome::Voxel` variants: `Air`, `Stone`, `Dirt`, `Grass`,
`Sand`, `Sandstone`, `Snow`, `Ice`, `Water`, `Gravel`, `Rock`, `Crystal`,
`Magma`, `Obsidian`.

> When voxels themselves are migrated to content (see `../Voxels/`), these
> string enum names will change to stable IDs / asset paths.

## Hot-reload

Editing any file in this directory while the editor is running causes the
crafting panel to rebuild live.  If every file is deleted or malformed the
panel falls back to the compiled-in recipe list in
`crates/atlas_voxel_planet/src/crafting.rs`, so the game always has *some*
working recipes.

## Rust types

Defined in [`atlas_assets::recipe`](../../../crates/atlas_assets/src/recipe.rs).
