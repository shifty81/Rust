/// Biome identifiers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    DeepOcean,
    ShallowOcean,
    Beach,
    Plains,
    Forest,
    TropicalForest,
    Desert,
    Savanna,
    Tundra,
    Arctic,
    Mountain,
    SnowPeak,
}

/// Voxel material types.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum Voxel {
    #[default]
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Grass = 3,
    Sand = 4,
    Sandstone = 5,
    Snow = 6,
    Ice = 7,
    Water = 8,
    Gravel = 9,
    Rock = 10,
}

impl Voxel {
    pub fn is_solid(self) -> bool {
        !matches!(self, Voxel::Air | Voxel::Water)
    }

    /// Return the raw `repr(u8)` byte for binary serialisation.
    #[inline]
    pub fn to_u8(self) -> u8 { self as u8 }

    /// Reconstruct a [`Voxel`] from a raw byte.  Unknown values become
    /// [`Voxel::Air`] so corrupted files degrade gracefully.
    #[inline]
    pub fn from_u8(b: u8) -> Self {
        match b {
            1  => Self::Stone,
            2  => Self::Dirt,
            3  => Self::Grass,
            4  => Self::Sand,
            5  => Self::Sandstone,
            6  => Self::Snow,
            7  => Self::Ice,
            8  => Self::Water,
            9  => Self::Gravel,
            10 => Self::Rock,
            _  => Self::Air,
        }
    }

    /// SRGB colour [r, g, b, a] used for vertex colouring.
    pub fn color(self) -> [f32; 4] {
        match self {
            Voxel::Air       => [0.0,  0.0,  0.0,  0.0],
            Voxel::Stone     => [0.45, 0.43, 0.40, 1.0],
            Voxel::Dirt      => [0.52, 0.36, 0.20, 1.0],
            Voxel::Grass     => [0.25, 0.58, 0.18, 1.0],
            Voxel::Sand      => [0.86, 0.80, 0.54, 1.0],
            Voxel::Sandstone => [0.78, 0.68, 0.40, 1.0],
            Voxel::Snow      => [0.92, 0.95, 1.00, 1.0],
            Voxel::Ice       => [0.72, 0.87, 0.95, 1.0],
            Voxel::Water     => [0.10, 0.40, 0.80, 0.75],
            Voxel::Gravel    => [0.50, 0.48, 0.45, 1.0],
            Voxel::Rock      => [0.35, 0.33, 0.30, 1.0],
        }
    }
}

/// Determine the biome for a surface point.
///
/// * `latitude`  – value in −1 (south pole) … +1 (north pole), derived from
///                 the Y component of the normalised surface direction.
/// * `altitude`  – metres above sea level (may be negative for ocean floor).
/// * `moisture`  – noise-derived value in 0…1.
pub fn classify_biome(latitude: f32, altitude: f32, moisture: f32) -> Biome {
    let temperature = ((1.0 - latitude.abs()) * 0.85 + 0.15)
        * (1.0 - (altitude.max(0.0) / 3_000.0).min(1.0));

    if altitude < -200.0 { return Biome::DeepOcean;    }
    if altitude < 0.0    { return Biome::ShallowOcean; }
    if altitude < 6.0    { return Biome::Beach;         }
    if altitude > 700.0  {
        return if temperature < 0.35 { Biome::SnowPeak } else { Biome::Mountain };
    }
    if temperature < 0.12 { return Biome::Arctic; }
    if temperature < 0.28 { return Biome::Tundra; }
    if temperature > 0.75 {
        if moisture > 0.55 { return Biome::TropicalForest; }
        if moisture < 0.25 { return Biome::Desert;         }
        return Biome::Savanna;
    }
    if moisture > 0.55 { return Biome::Forest; }
    if moisture < 0.20 { return Biome::Desert; }
    Biome::Plains
}

/// RGBA colour used for the planet overview mesh, given surface conditions.
pub fn biome_surface_color(biome: Biome, altitude: f32) -> [f32; 4] {
    let shade = 1.0 - (altitude.max(0.0) / 1_200.0).min(0.35);
    let [r, g, b, a] = match biome {
        Biome::DeepOcean     => [0.04, 0.12, 0.48, 1.0],
        Biome::ShallowOcean  => [0.08, 0.28, 0.70, 1.0],
        Biome::Beach         => [0.87, 0.82, 0.60, 1.0],
        Biome::Plains        => [0.40, 0.68, 0.25, 1.0],
        Biome::Forest        => [0.10, 0.42, 0.12, 1.0],
        Biome::TropicalForest=> [0.05, 0.48, 0.10, 1.0],
        Biome::Desert        => [0.85, 0.73, 0.38, 1.0],
        Biome::Savanna       => [0.65, 0.72, 0.28, 1.0],
        Biome::Tundra        => [0.55, 0.58, 0.48, 1.0],
        Biome::Arctic        => [0.90, 0.95, 1.00, 1.0],
        Biome::Mountain      => [0.50, 0.47, 0.42, 1.0],
        Biome::SnowPeak      => [0.96, 0.97, 1.00, 1.0],
    };
    [r * shade, g * shade, b * shade, a]
}

/// Choose what voxel material to place given biome and depth below surface.
pub fn voxel_for_depth(biome: Biome, depth: u32) -> Voxel {
    match biome {
        Biome::DeepOcean | Biome::ShallowOcean => match depth {
            0        => Voxel::Gravel,
            1..=3    => Voxel::Sand,
            _        => Voxel::Stone,
        },
        Biome::Beach => match depth {
            0..=2    => Voxel::Sand,
            3..=5    => Voxel::Gravel,
            _        => Voxel::Stone,
        },
        Biome::Desert => match depth {
            0..=3    => Voxel::Sand,
            4..=8    => Voxel::Sandstone,
            _        => Voxel::Stone,
        },
        Biome::Arctic => match depth {
            0        => Voxel::Snow,
            1..=3    => Voxel::Ice,
            _        => Voxel::Stone,
        },
        Biome::SnowPeak => match depth {
            0..=1    => Voxel::Snow,
            _        => Voxel::Rock,
        },
        Biome::Mountain => match depth {
            0        => Voxel::Rock,
            _        => Voxel::Stone,
        },
        Biome::Tundra => match depth {
            0..=1    => Voxel::Dirt,
            _        => Voxel::Stone,
        },
        _ => match depth {
            0        => Voxel::Grass,
            1..=3    => Voxel::Dirt,
            _        => Voxel::Stone,
        },
    }
}
