# Characters (`*.character.ron`) — **schema stub, not yet implemented**

Player / NPC body-part definitions.  Currently built from inline
`Cuboid` + `Sphere` shapes in
`crates/atlas_voxel_planet/src/character.rs`.

## Intended schema

```ron
(
    name: "Human",

    // Procedural body parts in local space, parent relationships encoded
    // by the `parent` field.  Later: replaced with a glTF asset ref.
    parts: [
        (name: "torso",  parent: None,            shape: Cuboid((0.4, 0.8, 0.25))),
        (name: "head",   parent: Some("torso"),   shape: Sphere(0.2),  offset: (0, 0.6, 0)),
        (name: "arm_l",  parent: Some("torso"),   shape: Cuboid((0.1, 0.6, 0.1)), offset: (-0.3, 0.2, 0)),
        (name: "arm_r",  parent: Some("torso"),   shape: Cuboid((0.1, 0.6, 0.1)), offset: ( 0.3, 0.2, 0)),
        (name: "leg_l",  parent: Some("torso"),   shape: Cuboid((0.12, 0.7, 0.12)), offset: (-0.12, -0.8, 0)),
        (name: "leg_r",  parent: Some("torso"),   shape: Cuboid((0.12, 0.7, 0.12)), offset: ( 0.12, -0.8, 0)),
    ],

    // Animation tuning used by animate_character_body in character.rs.
    anim: (
        stride_amplitude_deg: 25.0,
        breath_amplitude:     0.02,
    ),
)
```

## Migration notes
* Future `mesh: AssetRef("path/to/body.glb")` replaces `shape` when glTF
  character models land.
