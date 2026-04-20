//! Minimal LAN multiplayer foundation using a non-blocking UDP socket.
//!
//! # Architecture
//! A single `UdpSocket` is bound at startup (when `NetworkConfig.role ≠ Offline`)
//! and set to non-blocking mode.  Both host and clients share the same send/receive
//! path:
//!
//! * **Host** – binds `0.0.0.0:port`, broadcasts its own state to all known peers
//!   and relays other packets it has received.
//! * **Client** – binds `0.0.0.0:0` (ephemeral port), sends packets to the host
//!   address; receives packets from the host.
//!
//! # Wire format
//! Each packet is a fixed 32-byte [`NetPacket`]:
//! ```text
//! [0]     player_id: u8
//! [1]     flags: u8   (bit 0 = is_flying)
//! [2..=3] padding
//! [4..=7] pos.x: f32 LE
//! [8..=11] pos.y: f32 LE
//! [12..=15] pos.z: f32 LE
//! [16..=19] yaw: f32 LE
//! [20..=23] pitch: f32 LE
//! [24..=27] speed: f32 LE   (|velocity|)
//! [28..=31] padding
//! ```
//!
//! No new dependencies are required — only `std::net::UdpSocket`.
//!
//! # Controls / Usage
//! Expose `NetworkConfig` as a resource and set it before adding the plugin:
//! ```rust,ignore
//! app.insert_resource(NetworkConfig {
//!     role:       NetworkRole::Host,
//!     port:       7777,
//!     remote_addr: None,
//!     player_id:  0,
//!     player_name: "Player1".to_string(),
//! });
//! ```
//! For a client, set `role: NetworkRole::Client` and fill in `remote_addr`.

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;

use bevy::prelude::*;

use crate::components::*;

// ─────────────────────────────────────────────────────────────────────────────

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkConfig>()
            .init_resource::<NetworkState>()
            .add_systems(Startup, startup_network)
            .add_systems(
                Update,
                (
                    sync_player_to_network,
                    receive_network_updates,
                    despawn_stale_remotes,
                )
                    .chain(),
            );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Resources
// ─────────────────────────────────────────────────────────────────────────────

/// Whether this instance is a server, a client, or offline.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NetworkRole {
    #[default]
    Offline,
    /// Binds a fixed port, relays all player states to known peers.
    Host,
    /// Connects to a host; sends own state, receives relayed states.
    Client,
}

/// Multiplayer network configuration.  Set this resource before the app starts
/// (or modify it at runtime) to change the networking behaviour.
#[derive(Resource)]
pub struct NetworkConfig {
    pub role:        NetworkRole,
    /// UDP port to listen on (Host) or connect to (Client).
    pub port:        u16,
    /// For `Client` role: the host address, e.g. `"192.168.1.10"`.
    pub remote_host: Option<String>,
    /// 0-based index uniquely identifying this player instance.
    pub player_id:   u8,
    /// Display name sent in packets (first 12 bytes, ASCII).
    pub player_name: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            role:        NetworkRole::Offline,
            port:        7777,
            remote_host: None,
            player_id:   0,
            player_name: "Player".to_string(),
        }
    }
}

/// Runtime networking state: the bound socket + peer tracking.
#[derive(Resource, Default)]
pub struct NetworkState {
    /// Bound socket.  `None` when offline.
    pub socket:       Option<UdpSocket>,
    /// Host address (used by clients to direct-send).
    pub host_addr:    Option<SocketAddr>,
    /// All known peer addresses (for host-side relay).
    pub peers:        Vec<SocketAddr>,
    /// Accumulate time since last transmit (rate-limit to ~20 Hz).
    pub send_timer:   f32,
    /// Elapsed world time per remote player id, for staleness detection.
    pub last_seen:    HashMap<u8, f32>,
    /// Accumulated world time.
    pub world_time:   f32,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Components
// ─────────────────────────────────────────────────────────────────────────────

/// Tags a remotely-controlled player entity.
#[derive(Component)]
pub struct RemotePlayer {
    pub id:           u8,
    pub source_addr:  SocketAddr,
}

/// Interpolated display state for a remote player.
#[derive(Component, Default)]
pub struct RemotePlayerState {
    pub position: Vec3,
    pub yaw:      f32,
    pub pitch:    f32,
    pub speed:    f32,
    pub is_flying: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Wire format helpers
// ─────────────────────────────────────────────────────────────────────────────

const PACKET_SIZE: usize = 32;

struct NetPacket {
    player_id: u8,
    is_flying: bool,
    pos:   Vec3,
    yaw:   f32,
    pitch: f32,
    speed: f32,
}

impl NetPacket {
    fn encode(&self) -> [u8; PACKET_SIZE] {
        let mut buf = [0u8; PACKET_SIZE];
        buf[0] = self.player_id;
        buf[1] = if self.is_flying { 1 } else { 0 };
        buf[4..8].copy_from_slice(&self.pos.x.to_le_bytes());
        buf[8..12].copy_from_slice(&self.pos.y.to_le_bytes());
        buf[12..16].copy_from_slice(&self.pos.z.to_le_bytes());
        buf[16..20].copy_from_slice(&self.yaw.to_le_bytes());
        buf[20..24].copy_from_slice(&self.pitch.to_le_bytes());
        buf[24..28].copy_from_slice(&self.speed.to_le_bytes());
        buf
    }

    fn decode(buf: &[u8; PACKET_SIZE]) -> Self {
        let pos_x = f32::from_le_bytes(buf[4..8].try_into().unwrap_or_default());
        let pos_y = f32::from_le_bytes(buf[8..12].try_into().unwrap_or_default());
        let pos_z = f32::from_le_bytes(buf[12..16].try_into().unwrap_or_default());
        let yaw   = f32::from_le_bytes(buf[16..20].try_into().unwrap_or_default());
        let pitch = f32::from_le_bytes(buf[20..24].try_into().unwrap_or_default());
        let speed = f32::from_le_bytes(buf[24..28].try_into().unwrap_or_default());
        Self {
            player_id: buf[0],
            is_flying: buf[1] != 0,
            pos:   Vec3::new(pos_x, pos_y, pos_z),
            yaw,
            pitch,
            speed,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Startup
// ─────────────────────────────────────────────────────────────────────────────

fn startup_network(config: Res<NetworkConfig>, mut net: ResMut<NetworkState>) {
    if config.role == NetworkRole::Offline { return; }

    let bind_addr: SocketAddr = match config.role {
        NetworkRole::Host   => format!("0.0.0.0:{}", config.port).parse().unwrap(),
        NetworkRole::Client => "0.0.0.0:0".parse().unwrap(),
        NetworkRole::Offline => return,
    };

    match UdpSocket::bind(bind_addr) {
        Ok(socket) => {
            if let Err(e) = socket.set_nonblocking(true) {
                warn!("Multiplayer: could not set non-blocking: {e}");
                return;
            }
            // For a client, record the host's send address.
            if config.role == NetworkRole::Client {
                if let Some(host_str) = &config.remote_host {
                    let addr_str = format!("{host_str}:{}", config.port);
                    match SocketAddr::from_str(&addr_str) {
                        Ok(addr) => net.host_addr = Some(addr),
                        Err(e)   => warn!("Multiplayer: invalid host addr '{addr_str}': {e}"),
                    }
                }
            }
            info!(
                "Multiplayer socket bound on {:?} (role={:?})",
                socket.local_addr(), config.role
            );
            net.socket = Some(socket);
        }
        Err(e) => {
            warn!("Multiplayer: failed to bind socket on {bind_addr}: {e}");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Send own state
// ─────────────────────────────────────────────────────────────────────────────

fn sync_player_to_network(
    time:      Res<Time>,
    config:    Res<NetworkConfig>,
    mut net:   ResMut<NetworkState>,
    player_q:  Query<(&Transform, &PlayerState), With<Player>>,
) {
    if config.role == NetworkRole::Offline { return; }
    if net.socket.is_none() { return; }
    let Ok((tf, state)) = player_q.get_single() else { return };

    // Mutate timers before borrowing socket.
    let dt = time.delta_seconds();
    net.world_time += dt;
    net.send_timer  += dt;
    if net.send_timer < 0.05 { return; }
    net.send_timer = 0.0;

    let pkt = NetPacket {
        player_id: config.player_id,
        is_flying: state.is_flying,
        pos:       tf.translation,
        yaw:       state.yaw,
        pitch:     state.pitch,
        speed:     state.velocity.length(),
    };
    let buf = pkt.encode();

    // Borrow socket only after all mutations are done.
    let Some(socket) = &net.socket else { return };
    match config.role {
        NetworkRole::Host => {
            let peers: Vec<SocketAddr> = net.peers.clone();
            for peer in &peers {
                let _ = socket.send_to(&buf, peer);
            }
        }
        NetworkRole::Client => {
            if let Some(host) = net.host_addr {
                let _ = socket.send_to(&buf, host);
            }
        }
        NetworkRole::Offline => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Receive remote states
// ─────────────────────────────────────────────────────────────────────────────

fn receive_network_updates(
    config:        Res<NetworkConfig>,
    mut net:       ResMut<NetworkState>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut remote_q:  Query<(Entity, &RemotePlayer, &mut Transform, &mut RemotePlayerState)>,
) {
    if config.role == NetworkRole::Offline { return; }
    if net.socket.is_none() { return; }

    // Drain all pending datagrams into a local buffer first, so we hold no
    // borrow on `net` while we apply mutations below.
    let mut raw_packets: Vec<([u8; PACKET_SIZE], SocketAddr)> = Vec::new();
    {
        let Some(socket) = &net.socket else { return };
        let mut buf = [0u8; PACKET_SIZE];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, addr)) if n == PACKET_SIZE => {
                    raw_packets.push((buf, addr));
                }
                Ok(_) => { /* wrong size, drop */ }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => { warn!("Multiplayer recv error: {e}"); break; }
            }
        }
    } // socket borrow released

    let world_time = net.world_time;

    for (raw, src_addr) in raw_packets {
        let pkt = NetPacket::decode(&raw);

        // Skip our own packets (loopback from host relay).
        if pkt.player_id == config.player_id { continue; }

        // Register new peer on host.
        if config.role == NetworkRole::Host && !net.peers.contains(&src_addr) {
            info!("Multiplayer: new peer {src_addr}");
            net.peers.push(src_addr);
        }

        net.last_seen.insert(pkt.player_id, world_time);

        // Try to find existing remote entity.
        let mut found = false;
        for (_, rp, mut tf, mut rs) in &mut remote_q {
            if rp.id == pkt.player_id {
                tf.translation = tf.translation.lerp(pkt.pos, 0.3);
                rs.yaw       = pkt.yaw;
                rs.pitch     = pkt.pitch;
                rs.speed     = pkt.speed;
                rs.is_flying = pkt.is_flying;
                found = true;
                break;
            }
        }

        if !found {
            let body_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.80, 0.30, 0.20),
                perceptual_roughness: 0.85,
                ..default()
            });
            let head_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.87, 0.72, 0.53),
                perceptual_roughness: 0.80,
                ..default()
            });

            let body = commands.spawn((
                TransformBundle::from_transform(Transform::from_translation(pkt.pos)),
                VisibilityBundle::default(),
                RemotePlayer { id: pkt.player_id, source_addr: src_addr },
                RemotePlayerState {
                    position:  pkt.pos,
                    yaw:       pkt.yaw,
                    pitch:     pkt.pitch,
                    speed:     pkt.speed,
                    is_flying: pkt.is_flying,
                },
                Name::new(format!("RemotePlayer({})", pkt.player_id)),
            )).id();

            let torso = commands.spawn(PbrBundle {
                mesh:      meshes.add(Cuboid::new(0.5, 0.65, 0.3)),
                material:  body_mat.clone(),
                transform: Transform::from_translation(Vec3::new(0.0, 1.375, 0.0)),
                ..default()
            }).id();
            let head = commands.spawn(PbrBundle {
                mesh:      meshes.add(Sphere::new(0.2).mesh().uv(10, 8)),
                material:  head_mat,
                transform: Transform::from_translation(Vec3::new(0.0, 1.97, 0.0)),
                ..default()
            }).id();
            let leg_l = commands.spawn(PbrBundle {
                mesh:      meshes.add(Cuboid::new(0.2, 0.7, 0.2)),
                material:  body_mat.clone(),
                transform: Transform::from_translation(Vec3::new(-0.13, 0.35, 0.0)),
                ..default()
            }).id();
            let leg_r = commands.spawn(PbrBundle {
                mesh:      meshes.add(Cuboid::new(0.2, 0.7, 0.2)),
                material:  body_mat,
                transform: Transform::from_translation(Vec3::new(0.13, 0.35, 0.0)),
                ..default()
            }).id();

            commands.entity(body).push_children(&[torso, head, leg_l, leg_r]);
        }

        // On host: relay this packet to all other known peers.
        if config.role == NetworkRole::Host {
            let peers_copy: Vec<SocketAddr> = net.peers.iter()
                .filter(|&&p| p != src_addr)
                .copied()
                .collect();
            let encoded = pkt.encode();
            if let Some(sock) = &net.socket {
                for peer in peers_copy {
                    let _ = sock.send_to(&encoded, peer);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Stale peer cleanup
// ─────────────────────────────────────────────────────────────────────────────

/// Remove remote-player entities whose last packet arrived more than 5 s ago.
fn despawn_stale_remotes(
    config:     Res<NetworkConfig>,
    net:        Res<NetworkState>,
    mut commands: Commands,
    remote_q:   Query<(Entity, &RemotePlayer)>,
) {
    if config.role == NetworkRole::Offline { return; }

    for (entity, rp) in &remote_q {
        let last = net.last_seen.get(&rp.id).copied().unwrap_or(0.0);
        if net.world_time - last > 5.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}
