use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{inventory::ItemStack, registries::{ItemRegistry, Registries}};

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
    ItemStack(ItemStackJson)
}

#[derive(Serialize, Deserialize)]
pub struct ItemStackJson {
    id: String,
    count: i32,
}

#[derive(Clone, Debug)]
pub struct CraftingRecipe {
    pub pattern: Vec<Vec<ItemStack>>,
    pub result: ItemStack,
}

impl CraftingRecipe {
    pub fn from_json(mut json: CraftingRecipeJson, item_registry: &ItemRegistry) -> Self {
        json.substitutions.insert(" ".into(), CraftingRecipeJsonSubstitution::ItemID("nothing".into()));
        let pattern = json.pattern.into_iter().map(|row| row.chars().into_iter().map(|char| {
            match json.substitutions.get(&char.to_string()).expect(&format!("no substition for {char} in recipe")) {
                CraftingRecipeJsonSubstitution::ItemID(id) => ItemStack::new(item_registry.get_item(id).id, 1),
                CraftingRecipeJsonSubstitution::ItemStack(stack_json) => ItemStack { item: item_registry.get_item(&stack_json.id).id, count: stack_json.count }
            }
        }).collect()).collect();
        let result = match json.substitutions.get(&json.result).expect(&format!("no substition for {} in recipe", json.result)) {
            CraftingRecipeJsonSubstitution::ItemID(id) => ItemStack::new(item_registry.get_item(id).id, 1),
            CraftingRecipeJsonSubstitution::ItemStack(stack_json) => ItemStack { item: item_registry.get_item(&stack_json.id).id, count: stack_json.count }
        };
        Self {
            pattern,
            result,
        }
    }
}