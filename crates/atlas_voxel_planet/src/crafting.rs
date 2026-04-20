//! Crafting system.
//!
//! Press **C** to open and close the crafting panel.  Each recipe converts a
//! set of voxel ingredients drawn from the player's hotbar inventory into one
//! or more output voxels that are added to the hotbar.
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
//! * [`RECIPES`] is a static list of all [`Recipe`] values.
//! * [`CraftButton`] component on each button stores the recipe index.
//! * The `handle_craft_buttons` system performs the actual inventory transfer.

use bevy::prelude::*;

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

// ─────────────────────────────────────────────────────────────────────────────

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftingUiState>()
            .add_systems(Startup, setup_crafting_ui)
            .add_systems(
                Update,
                (
                    toggle_crafting_ui,
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

/// A single crafting recipe.
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

/// All available crafting recipes.
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

// ─────────────────────────────────────────────────────────────────────────────
//  Resource
// ─────────────────────────────────────────────────────────────────────────────

/// Whether the crafting panel is currently visible.
#[derive(Resource, Default)]
pub struct CraftingUiState {
    pub is_open: bool,
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

fn can_craft(recipe: &Recipe, inventory: &Inventory) -> bool {
    for &(ingredient, needed) in recipe.ingredients {
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

fn do_craft(recipe: &Recipe, inventory: &mut Inventory) {
    for &(ingredient, needed) in recipe.ingredients {
        if let Some(slot) = HOTBAR_VOXELS.iter().position(|&v| v == ingredient) {
            inventory.counts[slot] = inventory.counts[slot].saturating_sub(needed);
        }
    }
    for _ in 0..recipe.output_count {
        inventory.add(recipe.output_voxel);
    }
}

fn ingredient_text(recipe: &Recipe) -> String {
    recipe
        .ingredients
        .iter()
        .map(|(v, n)| format!("{}× {}", n, voxel_name(*v)))
        .collect::<Vec<_>>()
        .join(" + ")
}

// ─────────────────────────────────────────────────────────────────────────────
//  Systems
// ─────────────────────────────────────────────────────────────────────────────

fn setup_crafting_ui(mut commands: Commands) {
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
    )).id();

    // ── Header ───────────────────────────────────────────────────────────────
    let header = commands.spawn(TextBundle::from_section(
        "⚒ CRAFTING  [C to close]",
        TextStyle {
            font_size: 14.0,
            color:     Color::srgba(1.0, 0.9, 0.6, 1.0),
            ..default()
        },
    )).id();
    commands.entity(root).add_child(header);

    // ── Recipe rows ──────────────────────────────────────────────────────────
    for (i, recipe) in RECIPES.iter().enumerate() {
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
            recipe.name,
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
        commands.entity(root).add_child(row);
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
    mut panel_q: Query<&mut Visibility, With<CraftingPanel>>,
    mut btn_q:   Query<(&CraftButton, &mut BackgroundColor)>,
    mut label_q: Query<(&RecipeStatusText, &mut Text)>,
) {
    for mut vis in &mut panel_q {
        *vis = if state.is_open { Visibility::Inherited } else { Visibility::Hidden };
    }

    if !state.is_open { return; }

    for (btn, mut bg) in &mut btn_q {
        let craftable = can_craft(&RECIPES[btn.recipe_index], &inventory);
        *bg = if craftable {
            BackgroundColor(Color::srgba(0.18, 0.55, 0.18, 0.95))
        } else {
            BackgroundColor(Color::srgba(0.25, 0.25, 0.25, 0.70))
        };
    }

    for (label, mut text) in &mut label_q {
        let craftable = can_craft(&RECIPES[label.recipe_index], &inventory);
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
    state:       Res<CraftingUiState>,
    mut inventory: ResMut<Inventory>,
    btn_q:       Query<(&CraftButton, &Interaction), Changed<Interaction>>,
) {
    if !state.is_open { return; }

    for (btn, interaction) in &btn_q {
        if *interaction == Interaction::Pressed {
            let recipe = &RECIPES[btn.recipe_index];
            if can_craft(recipe, &inventory) {
                do_craft(recipe, &mut inventory);
            }
        }
    }
}
