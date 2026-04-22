# Structures (`*.structure.ron`) — **schema stub, not yet implemented**

A **Structure** is a higher-level composition of one or more prefabs, used
for procedural placement on the planet surface.  Ruins, watchtowers,
village blocks, abandoned outposts.

Difference vs. `../Prefabs/`: prefabs are a single template (one hut);
structures combine prefabs with placement rules (a *ring* of watchtowers
around a central hut with random rotation per instance).

## Intended schema

```ron
(
    name: "Frontier Outpost",

    // Where the procedural spawner may place this structure.
    biome_affinity: ["Plains", "Savanna"],

    // Weight relative to other structures in the same biome.
    spawn_weight: 1.0,

    // Prefab children + offsets.  Rotations picked randomly within the
    // given range (degrees) per instance.
    parts: [
        (prefab: "watchtower",  offset: ( 12, 0,  0), rotation_range: (0, 360)),
        (prefab: "watchtower",  offset: (-12, 0,  0), rotation_range: (0, 360)),
        (prefab: "hut",         offset: (  0, 0,  0), rotation_range: (0,   0)),
    ],

    // Min distance between instances of this structure kind on the planet.
    min_separation: 200.0,
)
```
