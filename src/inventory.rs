use bespoke_engine::resource_compiler::AtlasSection;
use itertools::Itertools;

use crate::{crafting::CraftingRecipe, items::{self, ItemID}, registries::Registries};

#[derive(PartialEq, Eq, Debug)]
pub struct Inventory {
    pub items: Vec<InventoryItemStack>,
    pub selected: usize,
}

impl Inventory {
    pub fn empty_size(size: usize, registries: &Registries) -> Self {
        Self {
            items: vec![InventoryItemStack::new(ItemStack::EMPTY, registries); size],
            selected: 0,
        }
    }

    pub fn selected_item(&self) -> &InventoryItemStack {
        &self.items[self.selected]
    }

    pub fn selected_item_mut(&mut self) -> &mut InventoryItemStack {
        &mut self.items[self.selected]
    }

    pub fn add(&mut self, item_stack: &InventoryItemStack) -> bool {
        for inventory_stack in &mut self.items {
            if inventory_stack.stack.item == item_stack.stack.item {
                inventory_stack.stack.count += item_stack.stack.count;
                return true;
            }
            if inventory_stack.stack.is_empty() {
                *inventory_stack = item_stack.clone();
                return true;
            }
        }
        return false;
    }

    pub fn calculate_crafting_result(&mut self, registries: &Registries) -> Option<CraftingRecipe> {
        let pattern = self.items[0..9].iter().chunks(3).into_iter().map(|row| row.map(|it| ItemStack { item: it.stack.item, count: 1 }).collect_vec()).collect_vec();
        let recipe = registries.crafting_registry.get_recipe_for_pattern(pattern, &registries);
        if let Some(recipe) = &recipe {
            self.items[9] = InventoryItemStack::new(recipe.result.clone(), registries);
        } else {
            self.items[9] = InventoryItemStack::new(ItemStack::EMPTY, registries);
        }
        recipe
    }

    pub fn take_crafting_result(&mut self, cursor_stack: &mut InventoryItemStack, registries: &Registries) {
        if let Some(recipe) = self.calculate_crafting_result(registries) {
            if cursor_stack.stack.is_empty() {
                std::mem::swap(cursor_stack, &mut self.items[9]);
            } else if cursor_stack.stack.item == self.items[9].stack.item {
                cursor_stack.stack.count += self.items[9].stack.count;
            } else {
                return;
            }
            let flattened_pattern = recipe.pattern.iter().flatten().collect_vec();
            for i in 0..9 {
                if flattened_pattern[i].tool {
                
                } else {
                    self.items[i].stack.count -= 1;
                }
                if self.items[i].stack.count <= 0 {
                    self.items[i] = InventoryItemStack::new(ItemStack::EMPTY, registries);
                }
            }
        }
    }
}

pub fn cursor_stack_interaction(cursor_stack: &mut InventoryItemStack, inventory_stack: &mut InventoryItemStack, interaction: InventoryInteraction, registries: &Registries) {
    if cursor_stack.stack.item == inventory_stack.stack.item {
        match interaction {
            InventoryInteraction::TakeOne => {
                cursor_stack.stack.count -= 1;
                inventory_stack.stack.count += 1;
            },
            InventoryInteraction::TakeStack => {
                inventory_stack.stack.count += cursor_stack.stack.count;
                cursor_stack.stack.count = 0;
            }
        };
    } else if !cursor_stack.stack.is_empty() && inventory_stack.stack.is_empty() {
        match interaction {
            InventoryInteraction::TakeOne => {
                *inventory_stack = InventoryItemStack::new(ItemStack::new(cursor_stack.stack.item, 1), registries);
                cursor_stack.stack.count -= 1;
            },
            InventoryInteraction::TakeStack => {
                std::mem::swap(cursor_stack, inventory_stack);
            }
        };
    } else if !inventory_stack.stack.is_empty() && cursor_stack.stack.is_empty() {
        match interaction {
            InventoryInteraction::TakeOne => {
                *cursor_stack = InventoryItemStack::new(ItemStack::new(inventory_stack.stack.item, 1), registries);
                inventory_stack.stack.count -= 1;
            },
            InventoryInteraction::TakeStack => {
                std::mem::swap(cursor_stack, inventory_stack);
            }
        };
    } else {
        std::mem::swap(cursor_stack, inventory_stack);
    }
    if cursor_stack.stack.count == 0 {
        *cursor_stack = InventoryItemStack::new(ItemStack::EMPTY, registries);
    }
    if inventory_stack.stack.count == 0 {
        *inventory_stack = InventoryItemStack::new(ItemStack::EMPTY, registries);
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InventoryItemStack {
    pub stack: ItemStack,
    pub atlas_section: AtlasSection,
}

impl InventoryItemStack {
    pub fn new(stack: ItemStack, registries: &Registries) -> Self {
        Self {
            atlas_section: registries.item_atlas_registry.get_item(&stack.item),
            stack,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct ItemStack {
    pub item: ItemID,
    pub count: i32,
}

impl ItemStack {
    pub const EMPTY: Self = ItemStack {
        item: items::NOTHING,
        count: 0,
    };

    pub fn new(item: ItemID, count: i32) -> Self {
        Self {
            count,
            item,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.item == items::NOTHING
    }
}

pub enum InventoryInteraction {
    TakeStack,
    TakeOne,
}