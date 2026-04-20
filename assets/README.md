# Atlas Engine — Asset Root

Bevy's `AssetServer` resolves paths relative to this directory when the game is
run from the workspace root (e.g. `cargo run -p atlas_runtime_app` or
`cargo run -p atlas_editor_app`).

## Layout

| Path                  | Used by                                                    |
|-----------------------|------------------------------------------------------------|
| `audio/ambient/*.ogg` | `atlas_voxel_planet::ambient_audio::AmbientAudioPlugin`    |

If an audio file is missing, the ambient-audio plugin logs a single `info!`
line on the first attempt and then stays silent for that biome — it is **not**
an error and the game runs fine without any audio assets at all.

## Adding biome music

Drop OGG Vorbis files into `audio/ambient/` with these names:

```
audio/ambient/plains.ogg
audio/ambient/forest.ogg
audio/ambient/desert.ogg
audio/ambient/arctic.ogg
audio/ambient/mountain.ogg
audio/ambient/ocean.ogg
audio/ambient/space.ogg
```

See `crates/atlas_voxel_planet/src/ambient_audio.rs` for the biome → track
mapping.
