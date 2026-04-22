# Creatures (`*.creature.ron`) — **schema stub, not yet implemented**

Creature kinds currently hardcoded in
`crates/atlas_voxel_planet/src/wildlife.rs` as the `CreatureKind` enum.

## Intended schema

```ron
(
    name: "Deer",

    // Which biomes this creature may spawn in.  Names mirror the current
    // Biome enum until biomes migrate to content.
    biome_affinity: ["Forest", "Plains"],

    // Base stats.
    stats: (
        max_health:   40.0,
        walk_speed:   4.5,
        sprint_speed: 9.0,
        hearing_range: 30.0,
    ),

    // AI parameters.
    ai: (
        wander_radius:   15.0,
        idle_time_range: (2.0, 6.0),
        flee_on_damage:  true,
    ),

    // Rendering — placeholder until character migration lands; currently
    // the procedural Cuboid+Sphere body is chosen from the enum.
    body: (
        kind: "Quadruped",
        scale: 1.0,
    ),
)
```
