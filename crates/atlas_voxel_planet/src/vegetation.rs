use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::biome::{classify_biome, Biome};
use crate::components::*;
use crate::config::*;
use crate::planet::terrain_radius_at;

pub struct VegetationPlugin;

impl Plugin for VegetationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_vegetation_around_player);
    }
}

fn spawn_vegetation_around_player(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    player_query:   Query<&Transform, With<Player>>,
    tree_query:     Query<(Entity, &Transform), (With<Tree>, Without<Player>)>,
    grass_query:    Query<(Entity, &Transform), (With<GrassDecoration>, Without<Player>)>,
    seed:           Res<NoiseSeed>,
    world_settings: Res<WorldSettings>,
) {
    let Ok(player_tf) = player_query.get_single() else { return };

    let player_pos    = player_tf.translation;
    let local_up      = player_pos.normalize_or_zero();
    let veg_radius    = world_settings.vegetation_radius;
    let despawn_r     = veg_radius * 1.5;
    let tree_chance   = world_settings.tree_spawn_chance;
    let grass_chance  = world_settings.grass_spawn_chance;

    let tree_prob_forest = tree_chance * 80.0;
    let tree_prob_plains = tree_chance * 30.0;
    let tree_prob_desert = tree_chance * 10.0;
    let tree_prob_tundra = tree_chance *  8.0;

    for (entity, tf) in &tree_query {
        if (tf.translation - player_pos).length() > despawn_r {
            commands.entity(entity).despawn_recursive();
        }
    }
    for (entity, tf) in &grass_query {
        if (tf.translation - player_pos).length() > despawn_r {
            commands.entity(entity).despawn_recursive();
        }
    }

    let existing_trees = tree_query
        .iter()
        .filter(|(_, tf)| (tf.translation - player_pos).length() < veg_radius)
        .count();
    let existing_grass = grass_query
        .iter()
        .filter(|(_, tf)| (tf.translation - player_pos).length() < veg_radius)
        .count();

    let need_trees = existing_trees < 80;
    let need_grass = existing_grass < 200;
    if !need_trees && !need_grass { return; }

    let mut rng = rand::thread_rng();

    for _ in 0..8 {
        let angle_h = rng.gen_range(0.0f32..2.0 * PI);
        let spread  = rng.gen_range(5.0f32..veg_radius);

        let ref_right = if local_up.abs().dot(Vec3::X) < 0.9 {
            Vec3::X.cross(local_up).normalize()
        } else {
            Vec3::Z.cross(local_up).normalize()
        };
        let ref_fwd  = local_up.cross(ref_right).normalize();
        let horiz    = ref_right * angle_h.cos() + ref_fwd * angle_h.sin();
        let cand_dir = (local_up + horiz * spread / PLANET_RADIUS).normalize();

        let surface_r = terrain_radius_at(cand_dir, seed.0);
        let altitude  = surface_r - PLANET_RADIUS;
        let latitude  = cand_dir.y;
        let moisture  = simple_moisture(cand_dir, seed.0);
        let biome     = classify_biome(latitude, altitude, moisture);

        if need_trees {
            match biome {
                Biome::Forest | Biome::TropicalForest => {
                    if rng.gen_range(0.0f32..1.0) < tree_prob_forest {
                        let pos = cand_dir * (surface_r + 0.5);
                        spawn_tree(&mut commands, &mut meshes, &mut materials, pos, cand_dir, TreeKind::Broadleaf, &mut rng);
                    }
                }
                Biome::Plains | Biome::Savanna => {
                    if rng.gen_range(0.0f32..1.0) < tree_prob_plains {
                        let pos = cand_dir * (surface_r + 0.5);
                        spawn_tree(&mut commands, &mut meshes, &mut materials, pos, cand_dir, TreeKind::Oak, &mut rng);
                    }
                }
                Biome::Desert => {
                    if rng.gen_range(0.0f32..1.0) < tree_prob_desert {
                        let pos = cand_dir * (surface_r + 0.5);
                        spawn_tree(&mut commands, &mut meshes, &mut materials, pos, cand_dir, TreeKind::Cactus, &mut rng);
                    }
                }
                Biome::Tundra => {
                    if rng.gen_range(0.0f32..1.0) < tree_prob_tundra {
                        let pos = cand_dir * (surface_r + 0.5);
                        spawn_tree(&mut commands, &mut meshes, &mut materials, pos, cand_dir, TreeKind::Pine, &mut rng);
                    }
                }
                _ => {}
            }
        }

        if need_grass {
            match biome {
                Biome::Plains | Biome::Forest | Biome::TropicalForest | Biome::Savanna => {
                    if rng.gen_range(0.0f32..1.0) < grass_chance {
                        let pos = cand_dir * (surface_r + 0.02);
                        spawn_grass(&mut commands, &mut meshes, &mut materials, pos, cand_dir, &mut rng);
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Copy)]
enum TreeKind { Broadleaf, Oak, Pine, Cactus }

fn spawn_tree(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position:  Vec3,
    surface_normal: Vec3,
    kind: TreeKind,
    rng:  &mut impl Rng,
) {
    let up      = surface_normal;
    let ref_vec = if up.abs().dot(Vec3::X) < 0.9 { Vec3::X } else { Vec3::Z };
    let right   = up.cross(ref_vec).normalize();
    let forward = right.cross(up).normalize();
    let rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));

    match kind {
        TreeKind::Broadleaf | TreeKind::Oak => {
            let trunk_h: f32  = rng.gen_range(3.0f32..7.0);
            let canopy_r: f32 = rng.gen_range(2.5f32..5.0);
            let canopy_g: f32 = if matches!(kind, TreeKind::Broadleaf) {
                rng.gen_range(0.35f32..0.60)
            } else {
                rng.gen_range(0.28f32..0.50)
            };

            let trunk_mat  = materials.add(StandardMaterial {
                base_color: Color::srgb(0.38, 0.24, 0.12),
                perceptual_roughness: 1.0,
                ..default()
            });
            let trunk_mesh = meshes.add(Cylinder::new(0.25, trunk_h).mesh().resolution(8).build());
            let canopy_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.12, canopy_g, 0.10),
                perceptual_roughness: 0.95,
                ..default()
            });
            let canopy_mesh = meshes.add(Sphere::new(canopy_r).mesh().uv(8, 6));

            commands
                .spawn((
                    TransformBundle::from_transform(Transform { translation: position, rotation, ..default() }),
                    VisibilityBundle::default(),
                    Tree,
                    Name::new("Tree"),
                ))
                .with_children(|p| {
                    p.spawn(PbrBundle {
                        mesh: trunk_mesh, material: trunk_mat,
                        transform: Transform::from_translation(Vec3::new(0.0, trunk_h * 0.5, 0.0)),
                        ..default()
                    });
                    p.spawn(PbrBundle {
                        mesh: canopy_mesh, material: canopy_mat,
                        transform: Transform::from_translation(Vec3::new(0.0, trunk_h + canopy_r * 0.6, 0.0)),
                        ..default()
                    });
                });
        }

        TreeKind::Pine => {
            let trunk_h: f32 = rng.gen_range(4.0f32..9.0);
            let cone_h = trunk_h * 1.4;

            let trunk_mat  = materials.add(StandardMaterial {
                base_color: Color::srgb(0.30, 0.18, 0.08),
                perceptual_roughness: 1.0,
                ..default()
            });
            let trunk_mesh = meshes.add(Cylinder::new(0.18, trunk_h).mesh().resolution(6).build());
            let cone_mat   = materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.28, 0.10),
                perceptual_roughness: 0.95,
                ..default()
            });
            let cone_mesh  = meshes.add(Cone { radius: trunk_h * 0.5, height: cone_h }.mesh().resolution(8).build());

            commands
                .spawn((
                    TransformBundle::from_transform(Transform { translation: position, rotation, ..default() }),
                    VisibilityBundle::default(),
                    Tree,
                    Name::new("Pine"),
                ))
                .with_children(|p| {
                    p.spawn(PbrBundle {
                        mesh: trunk_mesh, material: trunk_mat,
                        transform: Transform::from_translation(Vec3::new(0.0, trunk_h * 0.5, 0.0)),
                        ..default()
                    });
                    p.spawn(PbrBundle {
                        mesh: cone_mesh, material: cone_mat,
                        transform: Transform::from_translation(Vec3::new(0.0, trunk_h, 0.0)),
                        ..default()
                    });
                });
        }

        TreeKind::Cactus => {
            let height: f32 = rng.gen_range(1.8f32..4.5);
            let cactus_mat  = materials.add(StandardMaterial {
                base_color: Color::srgb(0.20, 0.48, 0.18),
                perceptual_roughness: 0.90,
                ..default()
            });
            let body_mesh = meshes.add(Cylinder::new(0.30, height).mesh().resolution(8).build());
            let arm_mesh  = meshes.add(Cylinder::new(0.18, height * 0.45).mesh().resolution(6).build());

            commands
                .spawn((
                    TransformBundle::from_transform(Transform { translation: position, rotation, ..default() }),
                    VisibilityBundle::default(),
                    Tree,
                    Name::new("Cactus"),
                ))
                .with_children(|p| {
                    p.spawn(PbrBundle {
                        mesh: body_mesh, material: cactus_mat.clone(),
                        transform: Transform::from_translation(Vec3::new(0.0, height * 0.5, 0.0)),
                        ..default()
                    });
                    p.spawn(PbrBundle {
                        mesh: arm_mesh, material: cactus_mat,
                        transform: Transform {
                            translation: Vec3::new(0.55, height * 0.55, 0.0),
                            rotation:    Quat::from_rotation_z(PI * 0.35),
                            ..default()
                        },
                        ..default()
                    });
                });
        }
    }
}

fn spawn_grass(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position:  Vec3,
    surface_normal: Vec3,
    rng: &mut impl Rng,
) {
    let up      = surface_normal;
    let ref_vec = if up.abs().dot(Vec3::X) < 0.9 { Vec3::X } else { Vec3::Z };
    let right   = up.cross(ref_vec).normalize();
    let forward = right.cross(up).normalize();
    let rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));

    let height: f32 = rng.gen_range(0.3f32..0.80);
    let width: f32  = rng.gen_range(0.2f32..0.45);
    let green: f32  = rng.gen_range(0.42f32..0.68);

    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, green, 0.12),
        perceptual_roughness: 0.85,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let blade_mesh = meshes.add(Cuboid::new(width, height, 0.025));

    commands
        .spawn((
            TransformBundle::from_transform(Transform { translation: position, rotation, ..default() }),
            VisibilityBundle::default(),
            GrassDecoration,
            Name::new("Grass"),
        ))
        .with_children(|p| {
            for i in 0..3_u32 {
                let blade_angle = i as f32 * PI / 3.0 + rng.gen_range(-0.25f32..0.25);
                p.spawn(PbrBundle {
                    mesh:     blade_mesh.clone(),
                    material: mat.clone(),
                    transform: Transform {
                        translation: Vec3::new(0.0, height * 0.5, 0.0),
                        rotation:    Quat::from_axis_angle(Vec3::Y, blade_angle),
                        ..default()
                    },
                    ..default()
                });
            }
        });
}

fn simple_moisture(dir: Vec3, seed: u32) -> f32 {
    use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
    let fbm: Fbm<Perlin> = Fbm::<Perlin>::new(seed.wrapping_add(7777))
        .set_octaves(3)
        .set_frequency(crate::config::MOISTURE_NOISE_SCALE)
        .set_lacunarity(2.1)
        .set_persistence(0.45);
    let v = fbm.get([dir.x as f64, dir.y as f64, dir.z as f64]) as f32;
    (v + 1.0) * 0.5
}
