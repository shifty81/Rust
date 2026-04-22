//! Crafting system.
//!
//! Press **C** to open and close the crafting panel.  Each recipe converts a
//! set of voxel ingredients drawn from the player's hotbar inventory into one
//! or more output voxels that are added to the hotbar.
//!
//! # Data-driven recipes
//!
//! Recipes are authored as RON files under `assets/Content/Recipes/` and
//! loaded at startup through [`atlas_assets::RecipeAsset`].  Editing a
//! `.recipe.ron` file while the editor is running triggers Bevy's asset
//! watcher, the [`RuntimeRecipes`] resource rebuilds, and the crafting panel
//! regenerates in place — no 25-minute engine rebuild required.  If the
//! `Content/Recipes/` directory is empty or unreachable, the panel falls
//! back to the compiled-in [`RECIPES`] list so standalone / test builds
//! still work without content on disk.
//!
//! # Panel layout
//! A semi-transparent panel appears on the right side of the screen.
//! Each row shows:
//! * **Recipe name** — e.g. "Compressed Stone"
//! * **Ingredients** — e.g. "3× Gravel"
//! * **Output** — e.g. "→ 2× Stone"
//! * **[Craft]** button — enabled (white) when the player has all ingredients;
//!   disabled (grey) otherwise.
//!
//! # Architecture
//! * [`CraftingUiState`] resource tracks whether the panel is open.
//! * [`RECIPES`] is the compiled-in fallback list of recipes.
//! * [`RuntimeRecipes`] is the *active* recipe table consumed by the UI;
//!   populated at startup from the fallback, then replaced when RON
//!   `RecipeAsset`s are loaded / modified / removed.
//! * [`CraftButton`] component on each button stores the recipe index.
//! * The `handle_craft_buttons` system performs the actual inventory transfer.

use bevy::prelude::*;

use atlas_assets::RecipeAsset;

use crate::biome::Voxel;
use crate::inventory::Inventory;

// ─────────────────────────────────────────────────────────────────────────────
//  Internal constant
// ─────────────────────────────────────────────────────────────────────────────

/// The voxels available in the hotbar, in slot order.
/// Kept in sync with the `HOTBAR_VOXELS` definition in `inventory.rs`.
const HOTBAR_VOXELS: [Voxel; 9] = [
    Voxel::Stone, Voxel::Dirt, Voxel::Grass, Voxel::Sand,
    Voxel::Sandstone, Voxel::Snow, Voxel::Gravel, Voxel::Crystal,
    Voxel::Obsidian,
];

/// Directory (relative to the Bevy asset root, which is `assets/`) that the
/// content scanner walks at startup looking for `*.recipe.ron` files.
const RECIPE_CONTENT_DIR: &str = "Content/Recipes";

// ─────────────────────────────────────────────────────────────────────────────

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftingUiState>()
            .init_resource::<RuntimeRecipes>()
            .init_resource::<RecipeFolder>()
            .add_systems(
                Startup,
                (populate_builtin_recipes, load_recipe_content, setup_crafting_ui)
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    toggle_crafting_ui,
                    rebuild_runtime_recipes_on_asset_change,
                    refill_crafting_panel_on_change,
                    update_crafting_ui,
                    handle_craft_buttons,
                )
                    .chain(),
            );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Recipes
// ─────────────────────────────────────────────────────────────────────────────

/// A single crafting recipe.  The compiled-in [`RECIPES`] list uses this type
/// with `&'static str` fields; the runtime table [`RuntimeRecipes`] stores
/// the owned [`OwnedRecipe`] equivalent so RON-loaded data can participate
/// without leaking strings.
pub struct Recipe {
    /// Human-readable name shown in the crafting panel.
    pub name: &'static str,
    /// Ingredient (voxel, count) pairs.  All must be satisfied simultaneously.
    pub ingredients: &'static [(Voxel, u32)],
    /// What the recipe produces.
    pub output_voxel: Voxel,
    /// How many output voxels are produced.
    pub output_count: u32,
}

/// Owned, heap-allocated variant of [`Recipe`] used for recipes loaded from
/// disk.  Functionally identical to [`Recipe`] but lives past the end of its
/// source asset file.
#[derive(Debug, Clone)]
pub struct OwnedRecipe {
    pub name:         String,
    pub ingredients:  Vec<(Voxel, u32)>,
    pub output_voxel: Voxel,
    pub output_count: u32,
}

impl OwnedRecipe {
    fn from_builtin(r: &Recipe) -> Self {
        Self {
            name:         r.name.to_string(),
            ingredients:  r.ingredients.to_vec(),
            output_voxel: r.output_voxel,
            output_count: r.output_count,
        }
    }

    /// Convert an authored [`RecipeAsset`] into the runtime form.  Unknown
    /// voxel names are logged and the recipe is skipped — returning `None`.
    fn from_asset(asset: &RecipeAsset, source_path: &str) -> Option<Self> {
        let mut ingredients = Vec::with_capacity(asset.ingredients.len());
        for ing in &asset.ingredients {
            match voxel_from_name(&ing.voxel) {
                Some(v) => ingredients.push((v, ing.count)),
                None => {
                    warn!(
                        "recipe '{}' in '{}' references unknown voxel '{}' — recipe skipped",
                        asset.name, source_path, ing.voxel
                    );
                    return None;
                }
            }
        }
        let output_voxel = match voxel_from_name(&asset.output.voxel) {
            Some(v) => v,
            None => {
                warn!(
                    "recipe '{}' in '{}' produces unknown voxel '{}' — recipe skipped",
                    asset.name, source_path, asset.output.voxel
                );
                return None;
            }
        };
        Some(Self {
            name:         asset.name.clone(),
            ingredients,
            output_voxel,
            output_count: asset.output.count,
        })
    }
}

/// Compiled-in fallback recipes.  Used before any `RecipeAsset`s load, or
/// permanently for builds that have no `Content/Recipes/` directory.
pub static RECIPES: &[Recipe] = &[
    Recipe {
        name:         "Compressed Stone",
        ingredients:  &[(Voxel::Gravel, 3)],
        output_voxel: Voxel::Stone,
        output_count: 2,
    },
    Recipe {
        name:         "Sandstone Slab",
        ingredients:  &[(Voxel::Sand, 3)],
        output_voxel: Voxel::Sandstone,
        output_count: 2,
    },
    Recipe {
        name:         "Rich Soil",
        ingredients:  &[(Voxel::Dirt, 2)],
        output_voxel: Voxel::Grass,
        output_count: 1,
    },
    Recipe {
        name:         "Crystal Refinement",
        ingredients:  &[(Voxel::Obsidian, 3)],
        output_voxel: Voxel::Crystal,
        output_count: 1,
    },
    Recipe {
        name:         "Stone Bricks",
        ingredients:  &[(Voxel::Stone, 2), (Voxel::Gravel, 1)],
        output_voxel: Voxel::Sandstone,
        output_count: 3,
    },
    Recipe {
        name:         "Snow Pack",
        ingredients:  &[(Voxel::Snow, 2), (Voxel::Crystal, 1)],
        output_voxel: Voxel::Ice,
        output_count: 2,
    },
];

/// Map a voxel enum-variant name (as authored in a `.recipe.ron` file) to
/// the engine's [`Voxel`] enum.  Returns `None` for unknown names so the
/// loader can skip malformed recipes instead of panicking.
///
/// **Transitional duplication of the `Voxel` enum:** when voxel types
/// themselves migrate to `assets/Content/Voxels/*.voxel.ron` (planned
/// follow-up PR), this function goes away entirely — recipes will
/// reference voxels by stable numeric ID / asset path, not by variant
/// name.  Adding a `strum`-style macro derive today would only bake in
/// an abstraction we're about to remove.
fn voxel_from_name(name: &str) -> Option<Voxel> {
    Some(match name {
        "Air"       => Voxel::Air,
        "Stone"     => Voxel::Stone,
        "Dirt"      => Voxel::Dirt,
        "Grass"     => Voxel::Grass,
        "Sand"      => Voxel::Sand,
        "Sandstone" => Voxel::Sandstone,
        "Snow"      => Voxel::Snow,
        "Ice"       => Voxel::Ice,
        "Water"     => Voxel::Water,
        "Gravel"    => Voxel::Gravel,
        "Rock"      => Voxel::Rock,
        "Crystal"   => Voxel::Crystal,
        "Magma"     => Voxel::Magma,
        "Obsidian"  => Voxel::Obsidian,
        _ => return None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
//  Resources
// ─────────────────────────────────────────────────────────────────────────────

/// Whether the crafting panel is currently visible.
#[derive(Resource, Default)]
pub struct CraftingUiState {
    pub is_open: bool,
}

/// The active recipe table the crafting UI reads from.  Starts out mirroring
/// the compiled-in [`RECIPES`] list, and is replaced wholesale once any
/// `RecipeAsset`s load from `assets/Content/Recipes/`.
#[derive(Resource, Default)]
pub struct RuntimeRecipes {
    pub recipes: Vec<OwnedRecipe>,
    /// `true` once at least one recipe has been loaded from disk; disables
    /// the compiled-in fallback for subsequent rebuilds.
    pub loaded_from_content: bool,
}

/// Strong handle to the `LoadedFolder` for the recipe content directory.
/// Bevy's asset server gives us a folder-level asset whose contents re-scan
/// when files are added, removed, or modified; that's what drives the
/// hot-reload path below.  Kept strong so the folder (and, transitively,
/// every contained `RecipeAsset`) stays alive for the editor's lifetime.
#[derive(Resource, Default)]
struct RecipeFolder {
    handle: Handle<bevy::asset::LoadedFolder>,
}

// ─────────────────────────────────────────────────────────────────────────────
//  UI components
// ─────────────────────────────────────────────────────────────────────────────

/// Marks the root node of the crafting panel.
#[derive(Component)]
pub struct CraftingPanel;

/// Marks a "Craft" button; stores the recipe it belongs to.
#[derive(Component)]
pub struct CraftButton {
    pub recipe_index: usize,
}

/// Marks the text node that shows recipe feasibility status inside a row.
#[derive(Component)]
pub struct RecipeStatusText {
    pub recipe_index: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn voxel_name(v: Voxel) -> &'static str {
    match v {
        Voxel::Air      => "Air",
        Voxel::Stone    => "Stone",
        Voxel::Dirt     => "Dirt",
        Voxel::Grass    => "Grass",
        Voxel::Sand     => "Sand",
        Voxel::Sandstone => "Sandstone",
        Voxel::Snow     => "Snow",
        Voxel::Ice      => "Ice",
        Voxel::Water    => "Water",
        Voxel::Gravel   => "Gravel",
        Voxel::Rock     => "Rock",
        Voxel::Crystal  => "Crystal",
        Voxel::Magma    => "Magma",
        Voxel::Obsidian => "Obsidian",
    }
}

fn can_craft(recipe: &OwnedRecipe, inventory: &Inventory) -> bool {
    for &(ingredient, needed) in &recipe.ingredients {
        let count = HOTBAR_VOXELS
            .iter()
            .position(|&v| v == ingredient)
            .map(|slot| inventory.counts[slot])
            .unwrap_or(0);
        if count < needed {
            return false;
        }
    }
    true
}

fn do_craft(recipe: &OwnedRecipe, inventory: &mut Inventory) {
    for &(ingredient, needed) in &recipe.ingredients {
        if let Some(slot) = HOTBAR_VOXELS.iter().position(|&v| v == ingredient) {
            inventory.counts[slot] = inventory.counts[slot].saturating_sub(needed);
        }
    }
    for _ in 0..recipe.output_count {
        inventory.add(recipe.output_voxel);
    }
}

fn ingredient_text(recipe: &OwnedRecipe) -> String {
    recipe
        .ingredients
        .iter()
        .map(|(v, n)| format!("{}× {}", n, voxel_name(*v)))
        .collect::<Vec<_>>()
        .join(" + ")
}

// ─────────────────────────────────────────────────────────────────────────────
//  Content loading
// ─────────────────────────────────────────────────────────────────────────────

/// Populate [`RuntimeRecipes`] from the compiled-in fallback list.  Runs
/// before `setup_crafting_ui` so the initial panel has rows immediately,
/// even on the very first frame before any RON assets have finished loading.
fn populate_builtin_recipes(mut runtime: ResMut<RuntimeRecipes>) {
    runtime.recipes = RECIPES.iter().map(OwnedRecipe::from_builtin).collect();
    runtime.loaded_from_content = false;
}

/// Ask the asset server to load every file under `Content/Recipes/` as a
/// [`LoadedFolder`].  Bevy walks the folder through its virtual filesystem
/// (respecting custom asset sources and web-compatible file access), and
/// re-scans it on disk events so files added **at runtime** show up without
/// an editor restart.  The contained typed assets are dispatched to the
/// registered `RonAssetLoader<RecipeAsset>` by extension.
///
/// If the directory is missing or empty we silently keep the compiled-in
/// fallback — this is expected in CI / headless contexts that don't ship a
/// content pack.  Bevy surfaces missing-folder warnings through the log,
/// so no extra error handling is needed here.
fn load_recipe_content(asset_server: Res<AssetServer>, mut folder: ResMut<RecipeFolder>) {
    folder.handle = asset_server.load_folder(RECIPE_CONTENT_DIR);
    info!(
        "crafting: requested recipe folder '{}' (hot-reload enabled)",
        RECIPE_CONTENT_DIR,
    );
}

/// React to any change in either the `LoadedFolder` (file added / removed)
/// or any individual `RecipeAsset` (file modified) by rebuilding the
/// [`RuntimeRecipes`] table from the currently-loaded assets.  If nothing
/// has finished loading yet (e.g. initial parse is still in flight), the
/// existing table is kept so the panel doesn't flicker to empty.
fn rebuild_runtime_recipes_on_asset_change(
    mut folder_events: EventReader<AssetEvent<bevy::asset::LoadedFolder>>,
    mut recipe_events: EventReader<AssetEvent<RecipeAsset>>,
    folder: Res<RecipeFolder>,
    folders: Res<Assets<bevy::asset::LoadedFolder>>,
    recipes: Res<Assets<RecipeAsset>>,
    mut runtime: ResMut<RuntimeRecipes>,
) {
    // Drain both readers; we only care *that* something changed, not which
    // event.  `count() > 0` drains them without the manual-flag idiom the
    // reviewer flagged.
    let folder_changed = folder_events.read().count() > 0;
    let recipe_changed = recipe_events.read().count() > 0;
    if !(folder_changed || recipe_changed) {
        return;
    }

    let Some(loaded_folder) = folders.get(&folder.handle) else {
        // Folder itself hasn't finished loading yet; next event will wake us.
        return;
    };

    let mut rebuilt: Vec<OwnedRecipe> = Vec::new();
    for untyped in &loaded_folder.handles {
        // The folder contains every file regardless of extension; typed
        // access filters naturally — `try_typed` succeeds only for
        // `.recipe.ron` handles (that's what our loader claims).
        let Ok(typed) = untyped.clone().try_typed::<RecipeAsset>() else { continue };
        let Some(asset) = recipes.get(&typed) else { continue };
        let path = typed.path().map(|p| p.to_string()).unwrap_or_else(|| "<unknown>".into());
        if let Some(owned) = OwnedRecipe::from_asset(asset, &path) {
            rebuilt.push(owned);
        }
    }

    if rebuilt.is_empty() {
        // No recipes loaded yet, or every file was malformed.  Keep whatever
        // we had rather than collapsing the panel.
        return;
    }

    // Stable alphabetical order by name so panel row indices are
    // deterministic across reloads.
    rebuilt.sort_by(|a, b| a.name.cmp(&b.name));

    runtime.recipes = rebuilt;
    runtime.loaded_from_content = true;
}

// ─────────────────────────────────────────────────────────────────────────────
//  Systems — panel lifecycle
// ─────────────────────────────────────────────────────────────────────────────

fn setup_crafting_ui(mut commands: Commands, runtime: Res<RuntimeRecipes>) {
    // ── Panel root (right side, hidden initially) ────────────────────────────
    let root = commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top:           Val::Px(0.0),
                right:         Val::Px(0.0),
                width:         Val::Px(260.0),
                height:        Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding:       UiRect::all(Val::Px(10.0)),
                row_gap:       Val::Px(6.0),
                overflow:      Overflow::clip_y(),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.88)),
            visibility: Visibility::Hidden,
            ..default()
        },
        CraftingPanel,
        crate::components::GameplayUiRoot,
    )).id();

    fill_crafting_panel(&mut commands, root, &runtime);
}

/// Spawn the header + one row per recipe as children of `panel`.
fn fill_crafting_panel(commands: &mut Commands, panel: Entity, runtime: &RuntimeRecipes) {
    // ── Header ───────────────────────────────────────────────────────────────
    let header = commands.spawn(TextBundle::from_section(
        "⚒ CRAFTING  [C to close]",
        TextStyle {
            font_size: 14.0,
            color:     Color::srgba(1.0, 0.9, 0.6, 1.0),
            ..default()
        },
    )).id();
    commands.entity(panel).add_child(header);

    // ── Recipe rows ──────────────────────────────────────────────────────────
    for (i, recipe) in runtime.recipes.iter().enumerate() {
        let row = commands.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(6.0)),
                row_gap:        Val::Px(3.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.80)),
            ..default()
        }).id();

        // Recipe name
        let name_txt = commands.spawn(TextBundle::from_section(
            recipe.name.clone(),
            TextStyle {
                font_size: 13.0,
                color:     Color::srgba(0.90, 0.90, 1.00, 1.0),
                ..default()
            },
        )).id();

        // Ingredients
        let ing_txt = commands.spawn(TextBundle::from_section(
            ingredient_text(recipe),
            TextStyle {
                font_size: 11.0,
                color:     Color::srgba(0.75, 0.85, 0.75, 1.0),
                ..default()
            },
        )).id();

        // Output
        let out_str = format!("→ {}× {}", recipe.output_count, voxel_name(recipe.output_voxel));
        let out_txt = commands.spawn(TextBundle::from_section(
            out_str,
            TextStyle {
                font_size: 11.0,
                color:     Color::srgba(0.70, 0.90, 1.00, 1.0),
                ..default()
            },
        )).id();

        // Craft button
        let btn = commands.spawn((
            ButtonBundle {
                style: Style {
                    padding:         UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                    justify_content: JustifyContent::Center,
                    align_self:      AlignSelf::FlexStart,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.20, 0.50, 0.20, 0.90)),
                ..default()
            },
            CraftButton { recipe_index: i },
        )).id();

        let btn_label = commands.spawn((
            TextBundle::from_section(
                "Craft",
                TextStyle {
                    font_size: 12.0,
                    color:     Color::WHITE,
                    ..default()
                },
            ),
            RecipeStatusText { recipe_index: i },
        )).id();
        commands.entity(btn).add_child(btn_label);

        commands.entity(row).push_children(&[name_txt, ing_txt, out_txt, btn]);
        commands.entity(panel).add_child(row);
    }
}

/// When [`RuntimeRecipes`] changes (content file added/edited/removed),
/// tear down the panel's existing rows and regenerate them so the new
/// recipe list is visible without restarting the editor.
fn refill_crafting_panel_on_change(
    mut commands: Commands,
    runtime: Res<RuntimeRecipes>,
    panel_q: Query<Entity, With<CraftingPanel>>,
) {
    if !runtime.is_changed() {
        return;
    }
    for panel in &panel_q {
        commands.entity(panel).despawn_descendants();
        fill_crafting_panel(&mut commands, panel, &runtime);
    }
}

/// Toggle panel visibility with the **C** key.
pub fn toggle_crafting_ui(
    keyboard:  Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CraftingUiState>,
) {
    if keyboard.just_pressed(KeyCode::KeyC) {
        state.is_open = !state.is_open;
    }
}

/// Sync panel visibility and update button colours based on inventory.
pub fn update_crafting_ui(
    state:     Res<CraftingUiState>,
    inventory: Res<Inventory>,
    runtime:   Res<RuntimeRecipes>,
    mut panel_q: Query<&mut Visibility, With<CraftingPanel>>,
    mut btn_q:   Query<(&CraftButton, &mut BackgroundColor)>,
    mut label_q: Query<(&RecipeStatusText, &mut Text)>,
) {
    for mut vis in &mut panel_q {
        *vis = if state.is_open { Visibility::Inherited } else { Visibility::Hidden };
    }

    if !state.is_open { return; }

    for (btn, mut bg) in &mut btn_q {
        let Some(recipe) = runtime.recipes.get(btn.recipe_index) else { continue };
        let craftable = can_craft(recipe, &inventory);
        *bg = if craftable {
            BackgroundColor(Color::srgba(0.18, 0.55, 0.18, 0.95))
        } else {
            BackgroundColor(Color::srgba(0.25, 0.25, 0.25, 0.70))
        };
    }

    for (label, mut text) in &mut label_q {
        let Some(recipe) = runtime.recipes.get(label.recipe_index) else { continue };
        let craftable = can_craft(recipe, &inventory);
        if let Some(s) = text.sections.first_mut() {
            s.value = if craftable {
                "Craft".to_string()
            } else {
                "Need more".to_string()
            };
            s.style.color = if craftable { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 1.0) };
        }
    }
}

/// Perform the craft when a button is pressed.
pub fn handle_craft_buttons(
    state:         Res<CraftingUiState>,
    runtime:       Res<RuntimeRecipes>,
    mut inventory: ResMut<Inventory>,
    btn_q:         Query<(&CraftButton, &Interaction), Changed<Interaction>>,
) {
    if !state.is_open { return; }

    for (btn, interaction) in &btn_q {
        if *interaction == Interaction::Pressed {
            let Some(recipe) = runtime.recipes.get(btn.recipe_index) else { continue };
            if can_craft(recipe, &inventory) {
                do_craft(recipe, &mut inventory);
            }
        }
    }
}
