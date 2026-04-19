use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::tasks::futures_lite::future;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use std::f32::consts::PI;

use crate::biome::{biome_surface_color, classify_biome, voxel_for_depth, Voxel};
use crate::components::*;
use crate::config::*;
use crate::RegenerateWorld;

// ---------------------------------------------------------------------------
//  Noise + material cache
// ---------------------------------------------------------------------------

/// Holds pre-built noise functions so they are not rebuilt every frame.
/// Rebuilt automatically when [`NoiseSeed`] or noise-related [`WorldSettings`]
/// change.
#[derive(Resource)]
pub struct NoiseCache {
    /// Seed the cache was built from.
    pub seed:             u32,
    /// Height FBM used for terrain.
    pub height_fbm:       Fbm<Perlin>,
    /// Moisture FBM used for biome classification.
    pub moisture_fbm:     Fbm<Perlin>,
    /// Shared material applied to every voxel chunk.
    pub chunk_mat:        Option<Handle<StandardMaterial>>,
    // ── Snapshot of generation params (used for change detection) ──────────
    pub terrain_noise_scale:  f64,
    pub moisture_noise_scale: f64,
    pub max_terrain_height:   f32,
    pub noise_octaves:        usize,
    pub noise_lacunarity:     f64,
    pub noise_persistence:    f64,
}

impl NoiseCache {
    pub fn build(seed: u32, settings: &WorldSettings) -> Self {
        let height_fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed)
            .set_octaves(settings.noise_octaves)
            .set_frequency(settings.terrain_noise_scale)
            .set_lacunarity(settings.noise_lacunarity)
            .set_persistence(settings.noise_persistence);

        let moisture_fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed.wrapping_add(7777))
            .set_octaves(settings.noise_octaves.min(5))
            .set_frequency(settings.moisture_noise_scale)
            .set_lacunarity(settings.noise_lacunarity)
            .set_persistence(settings.noise_persistence);

        Self {
            seed,
            height_fbm,
            moisture_fbm,
            chunk_mat: None,
            terrain_noise_scale:  settings.terrain_noise_scale,
            moisture_noise_scale: settings.moisture_noise_scale,
            max_terrain_height:   settings.max_terrain_height,
            noise_octaves:        settings.noise_octaves,
            noise_lacunarity:     settings.noise_lacunarity,
            noise_persistence:    settings.noise_persistence,
        }
    }

    /// Returns true if any noise parameter stored in `settings` differs from
    /// what was used to build this cache.
    pub fn params_match(&self, settings: &WorldSettings) -> bool {
        self.terrain_noise_scale  == settings.terrain_noise_scale
            && self.moisture_noise_scale == settings.moisture_noise_scale
            && self.max_terrain_height   == settings.max_terrain_height
            && self.noise_octaves        == settings.noise_octaves
            && self.noise_lacunarity     == settings.noise_lacunarity
            && self.noise_persistence    == settings.noise_persistence
    }
}

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkManager>()
            .init_resource::<NoiseSeed>()
            .init_resource::<WorldSettings>()
            .init_resource::<ChunkViewpoint>()
            .add_event::<RegenerateWorld>()
            .add_systems(Startup, (setup_planet, init_noise_cache).chain())
            .add_systems(
                Update,
                (
                    rebuild_noise_on_settings_change,
                    handle_regen_world,
                    unload_distant_chunks,
                    queue_chunks_around_viewpoint,
                    spawn_chunk_generation_tasks,
                    poll_chunk_generation_tasks,
                    remesh_dirty_chunks,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
//  Noise cache initialisation / rebuilding
// ---------------------------------------------------------------------------

fn init_noise_cache(
    seed:     Res<NoiseSeed>,
    settings: Res<WorldSettings>,
    mut commands: Commands,
) {
    commands.insert_resource(NoiseCache::build(seed.0, &settings));
}

/// Rebuild the noise cache whenever the noise seed or terrain noise parameters
/// change.  Also despawns non-manually-edited chunks so old geometry is not
/// mixed with new-seed geometry.
fn rebuild_noise_on_settings_change(
    seed:          Res<NoiseSeed>,
    settings:      Res<WorldSettings>,
    mut cache:     ResMut<NoiseCache>,
    mut chunk_mgr: ResMut<ChunkManager>,
    chunk_query:   Query<(Entity, &VoxelChunk), Without<ManuallyEdited>>,
    mut commands:  Commands,
) {
    let seed_changed     = seed.is_changed() && cache.seed != seed.0;
    let params_changed   = settings.is_changed() && !cache.params_match(&settings);

    if !seed_changed && !params_changed { return; }

    *cache = NoiseCache::build(seed.0, &settings);

    // Despawn non-manually-edited chunks so they are regenerated with new params.
    for (entity, chunk) in &chunk_query {
        commands.entity(entity).despawn_recursive();
        chunk_mgr.loaded.remove(&chunk.position);
    }
    chunk_mgr.pending.clear();
    chunk_mgr.pending_set.clear();
    chunk_mgr.in_flight.clear();
}

// ---------------------------------------------------------------------------

fn setup_planet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    seed: Res<NoiseSeed>,
    settings: Res<WorldSettings>,
) {
    let mesh = build_planet_mesh(seed.0, settings.max_terrain_height);
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(mesh),
            material: materials.add(StandardMaterial {
                perceptual_roughness: 0.95,
                metallic: 0.0,
                ..default()
            }),
            transform: Transform::default(),
            ..default()
        },
        Planet,
        Name::new("Planet"),
    ));

    // ── Ocean / sea-level sphere ─────────────────────────────────────────────
    // A slightly-transparent sphere at exactly sea-level radius.  Blended on
    // top of the planet overview so shallow oceans and beaches show beneath.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(SEA_LEVEL).mesh().uv(64, 32)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.04, 0.22, 0.62, 0.72),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.05,
                metallic: 0.1,
                reflectance: 0.6,
                double_sided: false,
                ..default()
            }),
            transform: Transform::default(),
            ..default()
        },
        crate::components::Ocean,
        Name::new("Ocean"),
    ));
}

fn build_planet_mesh(seed: u32, max_terrain_height: f32) -> Mesh {
    let height_fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed)
        .set_octaves(8)
        .set_frequency(TERRAIN_NOISE_SCALE)
        .set_lacunarity(2.0)
        .set_persistence(0.5);

    let moisture_fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed.wrapping_add(7777))
        .set_octaves(5)
        .set_frequency(MOISTURE_NOISE_SCALE)
        .set_lacunarity(2.1)
        .set_persistence(0.45);

    let lat_segs = PLANET_LAT_SEGS;
    let lon_segs = PLANET_LON_SEGS;

    let vert_count = ((lat_segs + 1) * (lon_segs + 1)) as usize;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vert_count);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(vert_count);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(vert_count);
    let mut colors:    Vec<[f32; 4]> = Vec::with_capacity(vert_count);

    for lat_i in 0..=lat_segs {
        let v   = lat_i as f32 / lat_segs as f32;
        let phi = PI * (v - 0.5);

        for lon_i in 0..=lon_segs {
            let u     = lon_i as f32 / lon_segs as f32;
            let theta = 2.0 * PI * u;

            let nx = phi.cos() * theta.cos();
            let ny = phi.sin();
            let nz = phi.cos() * theta.sin();

            let h_raw    = height_fbm.get([nx as f64, ny as f64, nz as f64]) as f32;
            let altitude = h_raw * max_terrain_height;
            let radius   = PLANET_RADIUS + altitude;

            let m_raw    = moisture_fbm.get([nx as f64, ny as f64, nz as f64]) as f32;
            let moisture = (m_raw + 1.0) * 0.5;

            let biome = classify_biome(ny, altitude, moisture);
            let color = biome_surface_color(biome, altitude);

            positions.push([nx * radius, ny * radius, nz * radius]);
            normals.push([nx, ny, nz]);
            uvs.push([u, v]);
            colors.push(color);
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity((lat_segs * lon_segs * 6) as usize);
    for lat_i in 0..lat_segs {
        for lon_i in 0..lon_segs {
            let row = lon_segs + 1;
            let v0  = lat_i * row + lon_i;
            let v1  = v0 + 1;
            let v2  = v0 + row;
            let v3  = v2 + 1;
            indices.extend_from_slice(&[v0, v2, v1, v1, v2, v3]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// ---------------------------------------------------------------------------
//  World regeneration
// ---------------------------------------------------------------------------

fn handle_regen_world(
    mut events:    EventReader<RegenerateWorld>,
    mut chunk_mgr: ResMut<ChunkManager>,
    // Only despawn chunks that have NOT been hand-edited by the user.
    chunk_query:   Query<(Entity, &VoxelChunk), Without<ManuallyEdited>>,
    mut commands:  Commands,
) {
    for _ev in events.read() {
        for (entity, chunk) in &chunk_query {
            commands.entity(entity).despawn_recursive();
            chunk_mgr.loaded.remove(&chunk.position);
        }
        chunk_mgr.pending.clear();
        chunk_mgr.pending_set.clear();
        chunk_mgr.in_flight.clear();
    }
}

// ---------------------------------------------------------------------------
//  Chunk management — driven by ChunkViewpoint (player OR editor camera)
// ---------------------------------------------------------------------------

fn unload_distant_chunks(
    mut commands:   Commands,
    viewpoint:      Res<ChunkViewpoint>,
    chunk_query:    Query<(Entity, &VoxelChunk)>,
    task_query:     Query<(Entity, &ChunkGenerationTask)>,
    mut chunk_mgr:  ResMut<ChunkManager>,
    world_settings: Res<WorldSettings>,
) {
    // No viewpoint yet (no camera/player has written to it).
    if viewpoint.0 == Vec3::ZERO { return; }

    let p  = viewpoint.0;
    let cs = (CHUNK_SIZE as f32) * VOXEL_SIZE;
    let view_chunk = IVec3::new(
        (p.x / cs).floor() as i32,
        (p.y / cs).floor() as i32,
        (p.z / cs).floor() as i32,
    );

    let rd        = world_settings.render_distance;
    let unload_sq = (rd + 2) * (rd + 2);

    // Unload finalised chunk entities
    for (entity, chunk) in &chunk_query {
        let d = chunk.position - view_chunk;
        if d.x * d.x + d.y * d.y + d.z * d.z > unload_sq {
            commands.entity(entity).despawn_recursive();
            chunk_mgr.loaded.remove(&chunk.position);
        }
    }

    // Also despawn stale task entities (chunk went out of range while in-flight)
    for (entity, task) in &task_query {
        let d = task.coord - view_chunk;
        if d.x * d.x + d.y * d.y + d.z * d.z > unload_sq {
            chunk_mgr.in_flight.remove(&task.coord);
            commands.entity(entity).despawn();
        }
    }
}

fn queue_chunks_around_viewpoint(
    viewpoint:      Res<ChunkViewpoint>,
    mut chunk_mgr:  ResMut<ChunkManager>,
    world_settings: Res<WorldSettings>,
) {
    if viewpoint.0 == Vec3::ZERO { return; }

    let p  = viewpoint.0;
    let cs = (CHUNK_SIZE as f32) * VOXEL_SIZE;
    let cx = (p.x / cs).floor() as i32;
    let cy = (p.y / cs).floor() as i32;
    let cz = (p.z / cs).floor() as i32;

    let rd = world_settings.render_distance;

    // Collect new coords with their squared distance so we can sort
    // nearest-first; players see close terrain appear before distant terrain.
    let mut new_coords: Vec<(IVec3, i32)> = Vec::new();
    for dx in -rd..=rd {
        for dy in -rd..=rd {
            for dz in -rd..=rd {
                let dist_sq = dx * dx + dy * dy + dz * dz;
                if dist_sq > rd * rd { continue; }
                let coord = IVec3::new(cx + dx, cy + dy, cz + dz);
                if !chunk_mgr.loaded.contains_key(&coord)
                    && !chunk_mgr.in_flight.contains(&coord)
                    && !chunk_mgr.pending_set.contains(&coord)
                {
                    new_coords.push((coord, dist_sq));
                }
            }
        }
    }

    // Sort descending so the closest coord ends up at the front after
    // push_front iterations.
    new_coords.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    for (coord, _) in new_coords {
        chunk_mgr.pending.push_front(coord);
        chunk_mgr.pending_set.insert(coord);
    }
}

// ────────────────────────────────────────────────────────────────────────────
//  Background-threaded chunk generation
// ────────────────────────────────────────────────────────────────────────────

/// Raw mesh geometry produced on a background thread (no GPU handles).
struct ChunkMeshData {
    positions:    Vec<[f32; 3]>,
    normals:      Vec<[f32; 3]>,
    colors:       Vec<[f32; 4]>,
    indices:      Vec<u32>,
    vertex_count: u32,
}

type ChunkTaskOutput = (IVec3, Vec<Voxel>, u32, Option<ChunkMeshData>);

/// Wraps a background chunk generation task together with the target coord.
#[derive(Component)]
pub struct ChunkGenerationTask {
    pub coord: IVec3,
    task:      Task<ChunkTaskOutput>,
}

/// Dequeue pending chunk positions and spawn background generation tasks.
/// At most `max_chunks_per_frame` new tasks are spawned each frame to avoid
/// flooding the thread pool with too much work at once.
fn spawn_chunk_generation_tasks(
    mut commands:   Commands,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    mut chunk_mgr:  ResMut<ChunkManager>,
    mut cache:      ResMut<NoiseCache>,
    world_settings: Res<WorldSettings>,
) {
    // Ensure the shared chunk material exists (main-thread only asset).
    if cache.chunk_mat.is_none() {
        cache.chunk_mat = Some(materials.add(StandardMaterial {
            perceptual_roughness: 0.92,
            metallic: 0.0,
            double_sided: false,
            ..default()
        }));
    }

    let pool = AsyncComputeTaskPool::get();
    let mut spawned = 0;

    while spawned < world_settings.max_chunks_per_frame {
        let Some(coord) = chunk_mgr.pending.pop_front() else { break };
        chunk_mgr.pending_set.remove(&coord);
        if chunk_mgr.loaded.contains_key(&coord) || chunk_mgr.in_flight.contains(&coord) {
            continue;
        }

        // Clone the FBMs so the closure is `'static` (Fbm<Perlin>: Clone+Send+Sync).
        let height_fbm   = cache.height_fbm.clone();
        let moisture_fbm = cache.moisture_fbm.clone();
        let max_h        = cache.max_terrain_height;

        let task = pool.spawn(async move {
            let (voxels, solid_count) =
                generate_chunk_data(coord, &height_fbm, &moisture_fbm, max_h);
            let mesh_data = build_chunk_mesh_data(&voxels);
            (coord, voxels, solid_count, mesh_data)
        });

        chunk_mgr.in_flight.insert(coord);
        commands.spawn(ChunkGenerationTask { coord, task });
        spawned += 1;
    }
}

/// Poll completed background tasks and materialise chunk entities.
fn poll_chunk_generation_tasks(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut chunk_mgr:  ResMut<ChunkManager>,
    cache:          Res<NoiseCache>,
    mut task_query: Query<(Entity, &mut ChunkGenerationTask)>,
) {
    let Some(mat) = cache.chunk_mat.clone() else { return };

    for (task_entity, mut ct) in &mut task_query {
        // Non-blocking poll — returns immediately if not done.
        let Some((coord, voxels, solid_count, mesh_data)) =
            future::block_on(future::poll_once(&mut ct.task))
        else {
            continue;
        };

        // Remove from in-flight set.
        chunk_mgr.in_flight.remove(&coord);
        // Despawn the temporary task entity.
        commands.entity(task_entity).despawn();

        // Chunk may have been loaded already (e.g. noise regen happened).
        if chunk_mgr.loaded.contains_key(&coord) {
            continue;
        }

        let cs     = (CHUNK_SIZE as f32) * VOXEL_SIZE;
        let origin = Vec3::new(
            coord.x as f32 * cs,
            coord.y as f32 * cs,
            coord.z as f32 * cs,
        );

        if let Some(data) = mesh_data {
            let vertex_count = data.vertex_count;
            let mesh         = mesh_from_data(data);
            let entity = commands.spawn((
                PbrBundle {
                    mesh:      meshes.add(mesh),
                    material:  mat.clone(),
                    transform: Transform::from_translation(origin),
                    ..default()
                },
                VoxelChunk { position: coord },
                VoxelData(voxels),
                ChunkInfo { solid_voxel_count: solid_count, vertex_count },
                Name::new(format!("Chunk({},{},{})", coord.x, coord.y, coord.z)),
            )).id();
            chunk_mgr.loaded.insert(coord, entity);
        } else {
            // Air-only chunk: invisible placeholder.
            let entity = commands.spawn((
                TransformBundle::from_transform(Transform::from_translation(origin)),
                VisibilityBundle::default(),
                VoxelChunk { position: coord },
                VoxelData(voxels),
                ChunkInfo { solid_voxel_count: 0, vertex_count: 0 },
                Name::new(format!("Chunk({},{},{})", coord.x, coord.y, coord.z)),
            )).id();
            chunk_mgr.loaded.insert(coord, entity);
        }
    }
}

/// Build raw mesh data on a background thread (no GPU resources).
fn build_chunk_mesh_data(voxels: &[Voxel]) -> Option<ChunkMeshData> {
    let cs = CHUNK_SIZE as i32;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut colors:    Vec<[f32; 4]> = Vec::new();
    let mut indices:   Vec<u32>      = Vec::new();

    // Face directions — order must match FACE_AO below.
    const DIRS: [(i32, i32, i32); 6] = [
        (1, 0, 0), (-1, 0, 0),
        (0, 1, 0), (0, -1, 0),
        (0, 0, 1), (0, 0, -1),
    ];

    // Per-face, per-vertex AO neighbour offsets: [edge0, edge1, corner].
    // Derived by computing which 3 voxels share each integer face-vertex
    // position on the external side of the face.  Both edge neighbours are
    // used in the standard AO formula; if both are solid the vertex is fully
    // occluded regardless of the corner.
    type AoTriplet   = [(i32,i32,i32); 3]; // [edge0, edge1, corner]
    type FaceAoTable = [AoTriplet; 4];     // one per quad vertex

    const FACE_AO: [FaceAoTable; 6] = [
        // +X
        [[(1,-1, 0),(1, 0,-1),(1,-1,-1)],[(1, 0,-1),(1, 1, 0),(1, 1,-1)],
         [(1, 1, 0),(1, 0, 1),(1, 1, 1)],[(1,-1, 0),(1, 0, 1),(1,-1, 1)]],
        // -X
        [[(-1,-1, 0),(-1, 0, 1),(-1,-1, 1)],[(-1, 0, 1),(-1, 1, 0),(-1, 1, 1)],
         [(-1, 0,-1),(-1, 1, 0),(-1, 1,-1)],[(-1,-1, 0),(-1, 0,-1),(-1,-1,-1)]],
        // +Y
        [[(-1, 1, 0),(0, 1,-1),(-1, 1,-1)],[(-1, 1, 0),(0, 1, 1),(-1, 1, 1)],
         [(0, 1, 1),(1, 1, 0),(1, 1, 1)],  [(0, 1,-1),(1, 1, 0),(1, 1,-1)]],
        // -Y
        [[(-1,-1, 0),(0,-1, 1),(-1,-1, 1)],[(-1,-1, 0),(0,-1,-1),(-1,-1,-1)],
         [(0,-1,-1),(1,-1, 0),(1,-1,-1)],  [(0,-1, 1),(1,-1, 0),(1,-1, 1)]],
        // +Z
        [[(0,-1, 1),(1, 0, 1),(1,-1, 1)],  [(0, 1, 1),(1, 0, 1),(1, 1, 1)],
         [(-1, 0, 1),(0, 1, 1),(-1, 1, 1)],[(-1, 0, 1),(0,-1, 1),(-1,-1, 1)]],
        // -Z
        [[(-1, 0,-1),(0,-1,-1),(-1,-1,-1)],[(-1, 0,-1),(0, 1,-1),(-1, 1,-1)],
         [(0, 1,-1),(1, 0,-1),(1, 1,-1)],  [(0,-1,-1),(1, 0,-1),(1,-1,-1)]],
    ];

    // Brightness per AO level (0 = two solid edges = darkest, 3 = fully lit).
    const AO_BRIGHT: [f32; 4] = [0.35, 0.60, 0.80, 1.00];

    for lx in 0..cs {
        for ly in 0..cs {
            for lz in 0..cs {
                let v = get_voxel(voxels, lx, ly, lz);
                if !v.is_solid() { continue; }

                let [r, g, b, a] = v.color();
                let ox = lx as f32;
                let oy = ly as f32;
                let oz = lz as f32;

                for (face_idx, (dx, dy, dz)) in DIRS.iter().enumerate() {
                    let (dx, dy, dz) = (*dx, *dy, *dz);
                    let neighbour = get_voxel(voxels, lx + dx, ly + dy, lz + dz);
                    if neighbour.is_solid() { continue; }

                    let face_verts   = face_vertices(ox, oy, oz, dx, dy, dz);
                    let normal       = [dx as f32, dy as f32, dz as f32];
                    let ao_table     = &FACE_AO[face_idx];

                    // Compute per-vertex AO and gather brightness values.
                    let mut ao_levels = [3u8; 4];
                    for (vi, [e0, e1, c]) in ao_table.iter().enumerate() {
                        let s0 = get_voxel(voxels, lx+e0.0, ly+e0.1, lz+e0.2).is_solid();
                        let s1 = get_voxel(voxels, lx+e1.0, ly+e1.1, lz+e1.2).is_solid();
                        let sc = get_voxel(voxels, lx+c.0,  ly+c.1,  lz+c.2 ).is_solid();
                        ao_levels[vi] = if s0 && s1 { 0 }
                                        else { 3 - (s0 as u8 + s1 as u8 + sc as u8) };
                    }

                    // Flip the quad diagonal when it reduces AO anisotropy.
                    let base = positions.len() as u32;
                    let flip = (ao_levels[0] as u16 + ao_levels[2] as u16)
                             < (ao_levels[1] as u16 + ao_levels[3] as u16);

                    for (vi, fv) in face_verts.iter().enumerate() {
                        let bright = AO_BRIGHT[ao_levels[vi] as usize];
                        positions.push(*fv);
                        normals.push(normal);
                        colors.push([r * bright, g * bright, b * bright, a]);
                    }

                    if flip {
                        indices.extend_from_slice(&[base, base+1, base+3, base+1, base+2, base+3]);
                    } else {
                        indices.extend_from_slice(&[base, base+1, base+2, base+2, base+3, base]);
                    }
                }
            }
        }
    }

    if positions.is_empty() { return None; }

    let vertex_count = positions.len() as u32;
    Some(ChunkMeshData { positions, normals, colors, indices, vertex_count })
}

/// Assemble a `Mesh` from pre-computed [`ChunkMeshData`] on the main thread.
fn mesh_from_data(data: ChunkMeshData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   data.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    data.colors);
    mesh.insert_indices(Indices::U32(data.indices));
    mesh
}

// ---------------------------------------------------------------------------
//  Terrain density
// ---------------------------------------------------------------------------

const CHUNK_VOL: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Returns the voxel data and the number of solid voxels in this chunk.
fn generate_chunk_data(
    coord:              IVec3,
    height_fbm:         &impl NoiseFn<f64, 3>,
    moisture_fbm:       &impl NoiseFn<f64, 3>,
    max_terrain_height: f32,
) -> (Vec<Voxel>, u32) {
    let mut voxels = vec![Voxel::Air; CHUNK_VOL];
    let mut solid  = 0u32;
    let cs = CHUNK_SIZE as i32;

    for lx in 0..cs {
        for ly in 0..cs {
            for lz in 0..cs {
                let wx = (coord.x * cs + lx) as f32;
                let wy = (coord.y * cs + ly) as f32;
                let wz = (coord.z * cs + lz) as f32;

                let world = Vec3::new(wx, wy, wz);
                let dist  = world.length();

                if dist < 1.0 {
                    let idx = voxel_index(lx as usize, ly as usize, lz as usize);
                    voxels[idx] = Voxel::Stone;
                    solid += 1;
                    continue;
                }

                let dir = world / dist;
                let nx  = dir.x as f64;
                let ny  = dir.y as f64;
                let nz  = dir.z as f64;

                let h_raw       = height_fbm.get([nx, ny, nz]) as f32;
                let terrain_r   = PLANET_RADIUS + h_raw * max_terrain_height;

                if dist > terrain_r + 1.0 {
                    if dist <= SEA_LEVEL {
                        let idx = voxel_index(lx as usize, ly as usize, lz as usize);
                        voxels[idx] = Voxel::Water;
                    }
                    continue;
                }

                let depth    = ((terrain_r - dist).max(0.0) as u32).min(255);
                let altitude = terrain_r - PLANET_RADIUS;
                let m_raw    = moisture_fbm.get([nx, ny, nz]) as f32;
                let moisture = (m_raw + 1.0) * 0.5;
                let biome    = classify_biome(dir.y, altitude, moisture);

                let idx = voxel_index(lx as usize, ly as usize, lz as usize);
                voxels[idx] = voxel_for_depth(biome, depth);
                solid += 1;
            }
        }
    }

    (voxels, solid)
}

#[inline]
fn voxel_index(x: usize, y: usize, z: usize) -> usize {
    chunk_voxel_index(x, y, z)
}

#[inline]
fn get_voxel(voxels: &[Voxel], x: i32, y: i32, z: i32) -> Voxel {
    let cs = CHUNK_SIZE as i32;
    if x < 0 || y < 0 || z < 0 || x >= cs || y >= cs || z >= cs {
        return Voxel::Air;
    }
    voxels[voxel_index(x as usize, y as usize, z as usize)]
}

// ---------------------------------------------------------------------------
//  Chunk remesh — rebuilds the mesh for any chunk flagged ChunkDirty
// ---------------------------------------------------------------------------

/// Whenever a chunk is marked [`ChunkDirty`] (e.g. by the voxel editing
/// tools), this system rebuilds its mesh from the stored [`VoxelData`] and
/// removes the dirty flag.
fn remesh_dirty_chunks(
    mut commands:    Commands,
    mut meshes:      ResMut<Assets<Mesh>>,
    mut dirty_chunks: Query<(Entity, &VoxelData, &mut Handle<Mesh>), With<ChunkDirty>>,
) {
    for (entity, voxel_data, mut mesh_handle) in &mut dirty_chunks {
        if let Some((mesh, _)) = build_chunk_mesh(&voxel_data.0) {
            *mesh_handle = meshes.add(mesh);
        }
        commands.entity(entity).remove::<ChunkDirty>();
    }
}

// ---------------------------------------------------------------------------
//  Chunk mesh builder — returns mesh + vertex count.
// ---------------------------------------------------------------------------

pub fn build_chunk_mesh(voxels: &[Voxel]) -> Option<(Mesh, u32)> {
    let data = build_chunk_mesh_data(voxels)?;
    let vertex_count = data.vertex_count;
    Some((mesh_from_data(data), vertex_count))
}

fn face_vertices(ox: f32, oy: f32, oz: f32, dx: i32, dy: i32, dz: i32) -> [[f32; 3]; 4] {
    match (dx, dy, dz) {
        (1, 0, 0)  => [[ox+1.,oy,    oz   ],[ox+1.,oy+1.,oz   ],[ox+1.,oy+1.,oz+1.],[ox+1.,oy,    oz+1.]],
        (-1, 0, 0) => [[ox,   oy,    oz+1.],[ox,   oy+1.,oz+1.],[ox,   oy+1.,oz   ],[ox,   oy,    oz   ]],
        (0, 1, 0)  => [[ox,   oy+1.,oz   ],[ox,   oy+1.,oz+1.],[ox+1.,oy+1.,oz+1.],[ox+1.,oy+1.,oz   ]],
        (0, -1, 0) => [[ox,   oy,    oz+1.],[ox,   oy,    oz   ],[ox+1.,oy,    oz   ],[ox+1.,oy,    oz+1.]],
        (0, 0, 1)  => [[ox+1.,oy,    oz+1.],[ox+1.,oy+1.,oz+1.],[ox,   oy+1.,oz+1.],[ox,   oy,    oz+1.]],
        _          => [[ox,   oy,    oz   ],[ox,   oy+1.,oz   ],[ox+1.,oy+1.,oz   ],[ox+1.,oy,    oz   ]],
    }
}

// ---------------------------------------------------------------------------
//  Public helper
// ---------------------------------------------------------------------------

/// Returns the terrain surface radius at the normalised world direction `dir`.
pub fn terrain_radius_at(dir: Vec3, seed: u32) -> f32 {
    let fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed)
        .set_octaves(8)
        .set_frequency(TERRAIN_NOISE_SCALE)
        .set_lacunarity(2.0)
        .set_persistence(0.5);
    let h_raw = fbm.get([dir.x as f64, dir.y as f64, dir.z as f64]) as f32;
    PLANET_RADIUS + h_raw * MAX_TERRAIN_HEIGHT
}

/// Compute a voxel index within a flat chunk array.
#[inline]
pub fn chunk_voxel_index(x: usize, y: usize, z: usize) -> usize {
    x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
}
