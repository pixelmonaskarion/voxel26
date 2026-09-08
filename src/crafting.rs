use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;

use crate::{inventory::ItemStack, items::ItemID, registries::{ItemRegistry, Registries, TagID, TagRegistry}};

#[derive(Serialize, Deserialize)]
pub struct CraftingRecipeJson {
    pub pattern: Vec<String>,
    pub result: String,
    pub substitutions: HashMap<String, CraftingRecipeJsonSubstitution>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum CraftingRecipeJsonSubstitution {
    ItemID(String),
    CraftingItemStack(CraftingItemStackJson),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct ItemStackData {
    pub id: String,
    pub count: i32,
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
    ItemID(ItemID),
    TagID(TagID),
}

impl CraftingIngredient {
    pub fn id(&self) -> CraftingIngredientID {
        match self {
            CraftingIngredient::ItemStack(stack) => CraftingIngredientID::ItemID(stack.item),
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
}

impl CraftingRecipe {
    pub fn from_json(mut json: CraftingRecipeJson, item_registry: &ItemRegistry, tag_registry: &TagRegistry) -> Self {
        json.substitutions.insert(" ".into(), CraftingRecipeJsonSubstitution::ItemID("nothing".into()));
        let get_ingredient = |id: &str, count: i32| {
            if id.starts_with("#") {
                CraftingIngredient::TagStack { tag: tag_registry.get_item_tag(&id[1..]).id, count }
            } else {
                CraftingIngredient::ItemStack(ItemStack::new(item_registry.get_item(id).id, count))
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
        }
    }

    pub fn matches(&self, pattern: &Vec<Vec<ItemStack>>, registries: &Registries) -> bool {
        for x in 0..3 {
            for y in 0..3 {
                let self_ingredient = &self.pattern[x][y].ingredient;
                let stack = &pattern[x][y];
                if !match self_ingredient {
                    CraftingIngredient::ItemStack(self_stack) => self_stack.item == stack.item,
                    CraftingIngredient::TagStack { tag, .. } => registries.tag_registry.get_item_tag(*tag).entries.contains(stack.item),
                } {
                    return false;
                }
            }
        }
        return true;
    }
}