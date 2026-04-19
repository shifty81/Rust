use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use std::f32::consts::PI;

use crate::biome::{biome_surface_color, classify_biome, voxel_for_depth, Voxel};
use crate::components::*;
use crate::config::*;
use crate::RegenerateWorld;

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkManager>()
            .init_resource::<NoiseSeed>()
            .init_resource::<WorldSettings>()
            .add_event::<RegenerateWorld>()
            .add_systems(Startup, setup_planet)
            .add_systems(
                Update,
                (
                    handle_regen_world,
                    unload_distant_chunks,
                    queue_chunks_around_player,
                    generate_pending_chunks,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
//  Planet overview mesh
// ---------------------------------------------------------------------------

fn setup_planet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    seed: Res<NoiseSeed>,
) {
    let mesh = build_planet_mesh(seed.0);
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
}

fn build_planet_mesh(seed: u32) -> Mesh {
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
            let altitude = h_raw * MAX_TERRAIN_HEIGHT;
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
    chunk_query:   Query<Entity, With<VoxelChunk>>,
    mut commands:  Commands,
) {
    for _ev in events.read() {
        for entity in &chunk_query {
            commands.entity(entity).despawn_recursive();
        }
        chunk_mgr.loaded.clear();
        chunk_mgr.pending.clear();
    }
}

// ---------------------------------------------------------------------------
//  Chunk management
// ---------------------------------------------------------------------------

fn unload_distant_chunks(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    chunk_query:  Query<(Entity, &VoxelChunk)>,
    mut chunk_mgr: ResMut<ChunkManager>,
    world_settings: Res<WorldSettings>,
) {
    let Ok(player_tf) = player_query.get_single() else { return };
    let p = player_tf.translation;

    let cs = (CHUNK_SIZE as f32) * VOXEL_SIZE;
    let player_chunk = IVec3::new(
        (p.x / cs).floor() as i32,
        (p.y / cs).floor() as i32,
        (p.z / cs).floor() as i32,
    );

    let rd        = world_settings.render_distance;
    let unload_sq = (rd + 2) * (rd + 2);

    for (entity, chunk) in &chunk_query {
        let d = chunk.position - player_chunk;
        if d.x * d.x + d.y * d.y + d.z * d.z > unload_sq {
            commands.entity(entity).despawn_recursive();
            chunk_mgr.loaded.remove(&chunk.position);
        }
    }
}

fn queue_chunks_around_player(
    player_query: Query<&Transform, With<Player>>,
    mut chunk_mgr: ResMut<ChunkManager>,
    world_settings: Res<WorldSettings>,
) {
    let Ok(player_tf) = player_query.get_single() else { return };
    let p = player_tf.translation;

    let cs = (CHUNK_SIZE as f32) * VOXEL_SIZE;
    let cx = (p.x / cs).floor() as i32;
    let cy = (p.y / cs).floor() as i32;
    let cz = (p.z / cs).floor() as i32;

    let rd = world_settings.render_distance;
    for dx in -rd..=rd {
        for dy in -rd..=rd {
            for dz in -rd..=rd {
                if dx * dx + dy * dy + dz * dz > rd * rd {
                    continue;
                }
                let coord = IVec3::new(cx + dx, cy + dy, cz + dz);
                if !chunk_mgr.loaded.contains_key(&coord)
                    && !chunk_mgr.pending.contains(&coord)
                {
                    chunk_mgr.pending.push_back(coord);
                }
            }
        }
    }
}

fn generate_pending_chunks(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    mut chunk_mgr:  ResMut<ChunkManager>,
    seed:           Res<NoiseSeed>,
    world_settings: Res<WorldSettings>,
) {
    let height_fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed.0)
        .set_octaves(8)
        .set_frequency(TERRAIN_NOISE_SCALE)
        .set_lacunarity(2.0)
        .set_persistence(0.5);

    let moisture_fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed.0.wrapping_add(7777))
        .set_octaves(5)
        .set_frequency(MOISTURE_NOISE_SCALE)
        .set_lacunarity(2.1)
        .set_persistence(0.45);

    let mat = materials.add(StandardMaterial {
        perceptual_roughness: 0.92,
        metallic: 0.0,
        double_sided: false,
        ..default()
    });

    let mut generated = 0;
    while generated < world_settings.max_chunks_per_frame {
        let Some(coord) = chunk_mgr.pending.pop_front() else { break };

        if chunk_mgr.loaded.contains_key(&coord) {
            continue;
        }

        let (voxels, solid_count) = generate_chunk_data(coord, &height_fbm, &moisture_fbm);
        let Some((mesh, vertex_count)) = build_chunk_mesh(&voxels) else {
            chunk_mgr.loaded.insert(coord, Entity::PLACEHOLDER);
            generated += 1;
            continue;
        };

        let cs     = (CHUNK_SIZE as f32) * VOXEL_SIZE;
        let origin = Vec3::new(
            coord.x as f32 * cs,
            coord.y as f32 * cs,
            coord.z as f32 * cs,
        );

        let entity = commands
            .spawn((
                PbrBundle {
                    mesh:      meshes.add(mesh),
                    material:  mat.clone(),
                    transform: Transform::from_translation(origin),
                    ..default()
                },
                VoxelChunk { position: coord },
                ChunkInfo { solid_voxel_count: solid_count, vertex_count },
                Name::new(format!("Chunk({},{},{})", coord.x, coord.y, coord.z)),
            ))
            .id();

        chunk_mgr.loaded.insert(coord, entity);
        generated += 1;
    }
}

// ---------------------------------------------------------------------------
//  Terrain density
// ---------------------------------------------------------------------------

const CHUNK_VOL: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Returns the voxel data and the number of solid voxels in this chunk.
fn generate_chunk_data(
    coord:        IVec3,
    height_fbm:   &impl NoiseFn<f64, 3>,
    moisture_fbm: &impl NoiseFn<f64, 3>,
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
                let terrain_r   = PLANET_RADIUS + h_raw * MAX_TERRAIN_HEIGHT;

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
    x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
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
//  Chunk mesh builder — returns mesh + vertex count.
// ---------------------------------------------------------------------------

fn build_chunk_mesh(voxels: &[Voxel]) -> Option<(Mesh, u32)> {
    let cs = CHUNK_SIZE as i32;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut colors:    Vec<[f32; 4]> = Vec::new();
    let mut indices:   Vec<u32>      = Vec::new();

    const DIRS: [(i32, i32, i32); 6] = [
        (1, 0, 0), (-1, 0, 0),
        (0, 1, 0), (0, -1, 0),
        (0, 0, 1), (0, 0, -1),
    ];

    for lx in 0..cs {
        for ly in 0..cs {
            for lz in 0..cs {
                let v = get_voxel(voxels, lx, ly, lz);
                if !v.is_solid() { continue; }

                let color = v.color();
                let ox = lx as f32;
                let oy = ly as f32;
                let oz = lz as f32;

                for (dx, dy, dz) in DIRS {
                    let neighbour = get_voxel(voxels, lx + dx, ly + dy, lz + dz);
                    if neighbour.is_solid() { continue; }

                    let base      = positions.len() as u32;
                    let face_verts = face_vertices(ox, oy, oz, dx, dy, dz);
                    let normal    = [dx as f32, dy as f32, dz as f32];

                    for fv in &face_verts {
                        positions.push(*fv);
                        normals.push(normal);
                        colors.push(color);
                    }
                    indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
                }
            }
        }
    }

    if positions.is_empty() { return None; }

    let vertex_count = positions.len() as u32;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    colors);
    mesh.insert_indices(Indices::U32(indices));
    Some((mesh, vertex_count))
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
