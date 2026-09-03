use cgmath::Vector2;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{items::{self, ItemID}, registries::Registries};

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

    pub fn calculate_crafting_result(&mut self, registries: &Registries) {
        let pattern = self.items[0..9].iter().chunks(3).into_iter().map(|row| row.map(|it| ItemStack { item: it.stack.item, count: 1 }).collect_vec()).collect_vec();
        if let Some(recipe) = registries.crafting_registry.get_recipe_for_pattern(pattern) {
            self.items[9] = InventoryItemStack::new(recipe.result, registries);
        } else {
            self.items[9] = InventoryItemStack::new(ItemStack::EMPTY, registries);
        }
    }

    pub fn take_crafting_result(&mut self, cursor_stack: &mut InventoryItemStack, registries: &Registries) {
        if cursor_stack.stack.is_empty() {
            std::mem::swap(cursor_stack, &mut self.items[9]);
            for i in 0..9 {
                self.items[i].stack.count -= 1;
                if self.items[i].stack.count <= 0 {
                    self.items[i] = InventoryItemStack::new(ItemStack::EMPTY, registries);
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InventoryItemStack {
    pub stack: ItemStack,
    pub atlas_coordinates: Vector2<u32>,
}

impl InventoryItemStack {
    pub fn new(stack: ItemStack, registries: &Registries) -> Self {
        Self {
            atlas_coordinates: registries.item_atlas_registry.get_item(&stack.item),
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