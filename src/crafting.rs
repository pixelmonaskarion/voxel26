use std::collections::HashMap;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;

use crate::{inventory::ItemStack, items::ItemIDRep, registries::{ItemRegistry, Registries, TagID, TagRegistry}};

#[derive(Serialize, Deserialize)]
pub struct CraftingRecipeJson {
    pub pattern: Vec<String>,
    pub result: String,
    pub crafting_type: String,
    pub substitutions: HashMap<String, CraftingRecipeJsonSubstitution>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum CraftingRecipeJsonSubstitution {
    ItemID(String),
    CraftingItemStack(CraftingItemStackJson),
}

#[serde_inline_default]
#[derive(Serialize, Deserialize)]
pub struct CraftingItemStackJson {
    pub id: String,
    #[serde_inline_default(1)]
    pub count: i32,
    #[serde_inline_default(false)]
    pub tool: bool,
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub enum CraftingIngredient {
    ItemStack(ItemStack),
    TagStack { tag: TagID, count: i32 },
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum CraftingIngredientID {
    ItemID(ItemIDRep),
    TagID(TagID),
}

pub const PLAYER_CRAFTING_TYPE: &'static str = "player_crafting";
pub const KILN_CRAFTING_TYPE: &'static str = "kiln";

impl CraftingIngredient {
    pub fn id(&self) -> CraftingIngredientID {
        match self {
            CraftingIngredient::ItemStack(stack) => CraftingIngredientID::ItemID(stack.item.clone()),
            CraftingIngredient::TagStack { tag, .. } => CraftingIngredientID::TagID(tag),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CraftingItemStack {
    pub ingredient: CraftingIngredient,
    pub tool: bool,
}

#[derive(Clone, Debug)]
pub struct CraftingRecipe {
    pub pattern: Vec<Vec<CraftingItemStack>>,
    pub result: ItemStack,
    pub crafting_type: String,
}

impl CraftingRecipe {
    pub fn from_json(mut json: CraftingRecipeJson, item_registry: &ItemRegistry, tag_registry: &TagRegistry) -> Self {
        json.substitutions.insert(" ".into(), CraftingRecipeJsonSubstitution::ItemID("nothing".into()));
        let get_ingredient = |id: &str, count: i32| {
            if id.starts_with("#") {
                CraftingIngredient::TagStack { tag: tag_registry.get_item_tag(&id[1..]).id, count }
            } else {
                CraftingIngredient::ItemStack(ItemStack::new(item_registry.get_item(id).id.into(), count))
            }
        };
        let get_substitution = |char| {
            match json.substitutions.get(&char).expect(&format!("no substition for {char} in recipe")) {
                CraftingRecipeJsonSubstitution::ItemID(id) => CraftingItemStack { ingredient: get_ingredient(id, 1), tool: false },
                CraftingRecipeJsonSubstitution::CraftingItemStack(stack) => CraftingItemStack { ingredient: get_ingredient(&stack.id, stack.count), tool: stack.tool },
            }
        };
        let pattern = json.pattern.into_iter().map(|row| row.chars().into_iter().map(|char| {
            get_substitution(char.to_string())
        }).collect()).collect();
        let CraftingIngredient::ItemStack(result) = get_substitution(json.result.clone()).ingredient else {
            panic!("crafting result must be an ItemStack, not {}", json.result);
        };
        Self {
            pattern,
            result,
            crafting_type: json.crafting_type,
        }
    }

    pub fn shrink_pattern(pattern: Vec<Vec<ItemStack>>) -> Vec<Vec<ItemStack>> {
        let Some(first_not_blank) = pattern.iter().find_position(|it| it.iter().any(|it| !it.is_empty())).map(|it| it.0) else {
            return pattern;
        };
        let last_not_blank = pattern.len()-pattern.iter().rev().find_position(|it| it.iter().any(|it| !it.is_empty())).map(|it| it.0).unwrap();
        let mut shrunk_pattern = pattern[first_not_blank..last_not_blank].to_vec();
        if shrunk_pattern.is_empty() || shrunk_pattern[0].is_empty() {
            return shrunk_pattern;
        }
        let mut blank_columns = vec![true; shrunk_pattern[0].len()];
        for x in 0..shrunk_pattern.len() {
            for y in 0..shrunk_pattern[x].len() {
                if !shrunk_pattern[x][y].is_empty() {
                    blank_columns[y] = false;
                }
            }
        }
        let first_not_blank = blank_columns.iter().find_position(|it| !**it).map(|it| it.0).unwrap();
        let last_not_blank = blank_columns.len()-1-blank_columns.iter().rev().find_position(|it| !**it).map(|it| it.0).unwrap();
        for x in 0..shrunk_pattern.len() {
            for y in (0..blank_columns.len()).rev() {
                if y < first_not_blank || y > last_not_blank {
                    shrunk_pattern[x].remove(y);
                }
            }
        }
        shrunk_pattern
    }

    pub fn matches(&self, pattern: &Vec<Vec<ItemStack>>, registries: &Registries) -> bool {
        if pattern.len() != self.pattern.len() || pattern[0].len() != self.pattern[0].len() {
            return false;
        }
        for x in 0..pattern.len() {
            for y in 0..pattern[x].len() {
                let self_ingredient = &self.pattern[x][y].ingredient;
                let stack = &pattern[x][y];
                if !match self_ingredient {
                    CraftingIngredient::ItemStack(self_stack) => self_stack.item == stack.item,
                    CraftingIngredient::TagStack { tag, .. } => registries.tag_registry.get_item_tag(*tag).entries.iter().any(|it| it == &stack.item),
                } {
                    return false;
                }
            }
        }
        return true;
    }
}