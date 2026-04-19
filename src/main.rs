use bevy::prelude::*;

mod biome;
mod components;
mod config;

mod atmosphere;
mod planet;
mod player;
mod solar_system;
mod vegetation;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Voxel Planet — Rust".into(),
                    resolution: (1280.0, 720.0).into(),
                    ..default()
                }),
                ..default()
            }),
        )
        // Ordered: solar system first (spawns lighting), then planet geometry,
        // then player (needs terrain_radius_at), then atmosphere, then vegetation.
        .add_plugins((
            solar_system::SolarSystemPlugin,
            planet::PlanetPlugin,
            player::PlayerPlugin,
            atmosphere::AtmospherePlugin,
            vegetation::VegetationPlugin,
        ))
        .run();
}
