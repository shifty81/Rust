//! `world_io` — save and load the manually-edited voxel world state.
//!
//! # File format  (`.voxelworld`)
//!
//! ```text
//! [4 bytes]  magic  "ATLV"
//! [4 bytes]  version  u32 LE  (currently 1)
//! [4 bytes]  chunk_count  u32 LE
//! for each chunk:
//!   [4 bytes]  coord.x  i32 LE
//!   [4 bytes]  coord.y  i32 LE
//!   [4 bytes]  coord.z  i32 LE
//!   [4096 bytes]  voxel_data  one byte per voxel (Voxel::to_u8 / from_u8)
//! ```
//!
//! Only chunks tagged [`ManuallyEdited`] are saved; procedurally generated
//! terrain is recreated from the noise seed on load.

use std::io::{Read, Write};
use std::path::PathBuf;

use bevy::prelude::*;

use crate::biome::Voxel;
use crate::components::{
    ChunkInfo, ChunkManager, ManuallyEdited, VoxelChunk, VoxelData,
};
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::planet::NoiseCache;

const MAGIC:   &[u8; 4] = b"ATLV";
const VERSION: u32       = 1;
const CHUNK_VOL: usize   = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

/// Send this event to write all [`ManuallyEdited`] chunks to `path`.
#[derive(Event)]
pub struct SaveWorldRequest(pub PathBuf);

/// Send this event to load saved chunk overrides from `path`.
/// Spawns the loaded chunks with [`ManuallyEdited`] so they survive regen.
#[derive(Event)]
pub struct LoadWorldRequest(pub PathBuf);

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct WorldIoPlugin;

impl Plugin for WorldIoPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveWorldRequest>()
            .add_event::<LoadWorldRequest>()
            .add_systems(Update, (handle_save, handle_load).chain());
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Save
// ────────────────────────────────────────────────────────────────────────────

fn handle_save(
    mut events:  EventReader<SaveWorldRequest>,
    edited_q:    Query<(&VoxelChunk, &VoxelData), With<ManuallyEdited>>,
) {
    for ev in events.read() {
        let path = &ev.0;
        let chunks: Vec<(&VoxelChunk, &VoxelData)> = edited_q.iter().collect();

        match save_world(path, &chunks) {
            Ok(n)  => info!("WorldIO: saved {} edited chunk(s) → {}", n, path.display()),
            Err(e) => error!("WorldIO: save failed: {e}"),
        }
    }
}

fn save_world(
    path:   &std::path::Path,
    chunks: &[(&VoxelChunk, &VoxelData)],
) -> std::io::Result<usize> {
    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);

    // Header
    w.write_all(MAGIC)?;
    w.write_all(&VERSION.to_le_bytes())?;
    w.write_all(&(chunks.len() as u32).to_le_bytes())?;

    for (chunk, vd) in chunks {
        let c = chunk.position;
        w.write_all(&c.x.to_le_bytes())?;
        w.write_all(&c.y.to_le_bytes())?;
        w.write_all(&c.z.to_le_bytes())?;

        // Voxel bytes — pad / truncate to exactly CHUNK_VOL
        let data = &vd.0;
        let write_len = data.len().min(CHUNK_VOL);
        let bytes: Vec<u8> = data[..write_len].iter().map(|v| v.to_u8()).collect();
        w.write_all(&bytes)?;
        // Zero-pad if shorter (shouldn't happen but guard against corruption)
        for _ in write_len..CHUNK_VOL {
            w.write_all(&[0u8])?;
        }
    }

    w.flush()?;
    Ok(chunks.len())
}

// ────────────────────────────────────────────────────────────────────────────
// Load
// ────────────────────────────────────────────────────────────────────────────

fn handle_load(
    mut events:    EventReader<LoadWorldRequest>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut chunk_mgr: ResMut<ChunkManager>,
    cache:         Res<NoiseCache>,
) {
    for ev in events.read() {
        let path = &ev.0;
        match load_world(path) {
            Ok(records) => {
                let n = records.len();
                for (coord, voxels) in records {
                    // Skip if already loaded (happens if called twice)
                    if chunk_mgr.loaded.contains_key(&coord) {
                        continue;
                    }

                    let mat = match &cache.chunk_mat {
                        Some(m) => m.clone(),
                        None    => {
                            warn!("WorldIO: chunk material not yet created; skipping chunk {coord}");
                            continue;
                        }
                    };

                    let cs     = (CHUNK_SIZE as f32) * VOXEL_SIZE;
                    let origin = Vec3::new(
                        coord.x as f32 * cs,
                        coord.y as f32 * cs,
                        coord.z as f32 * cs,
                    );

                    if let Some((mesh, vertex_count)) = crate::planet::build_chunk_mesh(&voxels) {
                        let solid_count = voxels.iter().filter(|v| v.is_solid()).count() as u32;
                        let entity = commands.spawn((
                            PbrBundle {
                                mesh: meshes.add(mesh),
                                material: mat,
                                transform: Transform::from_translation(origin),
                                ..default()
                            },
                            VoxelChunk { position: coord },
                            VoxelData(voxels),
                            ManuallyEdited,
                            ChunkInfo { solid_voxel_count: solid_count, vertex_count },
                            Name::new(format!("Chunk({},{},{})", coord.x, coord.y, coord.z)),
                        )).id();
                        chunk_mgr.loaded.insert(coord, entity);
                    } else {
                        // All-air chunk: still load it so re-save works
                        let entity = commands.spawn((
                            TransformBundle::from_transform(Transform::from_translation(origin)),
                            VisibilityBundle::default(),
                            VoxelChunk { position: coord },
                            VoxelData(voxels),
                            ManuallyEdited,
                            ChunkInfo { solid_voxel_count: 0, vertex_count: 0 },
                            Name::new(format!("Chunk({},{},{})", coord.x, coord.y, coord.z)),
                        )).id();
                        chunk_mgr.loaded.insert(coord, entity);
                    }
                }
                info!("WorldIO: loaded {n} edited chunk(s) from {}", path.display());
            }
            Err(e) => error!("WorldIO: load failed: {e}"),
        }
    }
}

/// Returns `(coord, voxels)` for every chunk record in the file.
fn load_world(path: &std::path::Path) -> std::io::Result<Vec<(IVec3, Vec<Voxel>)>> {
    let file = std::fs::File::open(path)?;
    let mut r = std::io::BufReader::new(file);

    // Magic
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a .voxelworld file",
        ));
    }

    // Version
    let mut ver_buf = [0u8; 4];
    r.read_exact(&mut ver_buf)?;
    let version = u32::from_le_bytes(ver_buf);
    if version != VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsupported .voxelworld version {version}"),
        ));
    }

    // Chunk count
    let mut count_buf = [0u8; 4];
    r.read_exact(&mut count_buf)?;
    let count = u32::from_le_bytes(count_buf) as usize;

    let mut records = Vec::with_capacity(count);
    let mut coord_buf = [0u8; 4];
    let mut voxel_buf = vec![0u8; CHUNK_VOL];

    for _ in 0..count {
        r.read_exact(&mut coord_buf)?;
        let x = i32::from_le_bytes(coord_buf);
        r.read_exact(&mut coord_buf)?;
        let y = i32::from_le_bytes(coord_buf);
        r.read_exact(&mut coord_buf)?;
        let z = i32::from_le_bytes(coord_buf);

        r.read_exact(&mut voxel_buf)?;
        let voxels: Vec<Voxel> = voxel_buf.iter().map(|&b| Voxel::from_u8(b)).collect();

        records.push((IVec3::new(x, y, z), voxels));
    }

    Ok(records)
}
