use std::ops::{AddAssign, SubAssign};

use itertools::Itertools;

use crate::{crafting::{CraftingRecipe, PLAYER_CRAFTING_TYPE}, items::{self, ItemIDRep}, registries::Registries};

#[derive(PartialEq, Eq, Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Inventory {
    pub items: Vec<ItemStack>,
    pub selected: usize,
}

impl Inventory {
    pub fn empty_size(size: usize) -> Self {
        Self {
            items: vec![ItemStack::empty(); size],
            selected: 0,
        }
    }

    pub fn selected_item(&self) -> &ItemStack {
        &self.items[self.selected]
    }

    pub fn selected_item_mut(&mut self) -> &mut ItemStack {
        &mut self.items[self.selected]
    }

    pub fn add(&mut self, item_stack: &ItemStack) -> bool {
        for inventory_stack in &mut self.items {
            if inventory_stack.item == item_stack.item {
                inventory_stack.count += item_stack.count;
                return true;
            }
            if inventory_stack.is_empty() {
                *inventory_stack = item_stack.clone();
                return true;
            }
        }
        return false;
    }

    pub fn calculate_crafting_result(&mut self, registries: &Registries) -> Option<CraftingRecipe> {
        let pattern = self.items[0..9].iter().chunks(3).into_iter().map(|row| row.map(|it| ItemStack { item: it.item.clone(), count: 1 }).collect_vec()).collect_vec();
        let recipe = registries.crafting_registry.get_recipe_for_pattern(pattern, PLAYER_CRAFTING_TYPE, &registries);
        if let Some(recipe) = &recipe {
            self.items[9] = recipe.result.clone();
        } else {
            self.items[9] = ItemStack::empty();
        }
        recipe
    }

    pub fn take_crafting_result(&mut self, cursor_stack: &mut ItemStack, registries: &Registries) {
        if let Some(recipe) = self.calculate_crafting_result(registries) {
            if cursor_stack.is_empty() {
                std::mem::swap(cursor_stack, &mut self.items[9]);
            } else if cursor_stack.item == self.items[9].item {
                *cursor_stack += self.items[9].count;
            } else {
                return;
            }
            let mut grid_start_x = 4;
            let mut grid_start_y = 4;
            'x: for x in 0..3 {
                for y in 0..3 {
                    if !self.items[x*3+y].is_empty() {
                        grid_start_x = grid_start_x.min(x);
                        grid_start_y = grid_start_y.min(y);
                        break 'x;
                    }
                }
            }
            for x in 0..recipe.pattern.len() {
                for y in 0..recipe.pattern[x].len() {
                    if recipe.pattern[x][y].tool {
                    
                    } else {
                        self.items[(grid_start_x+x)*3+grid_start_y+y] -= 1;
                    }
                }
            }
        }
    }
}

pub fn cursor_stack_interaction(cursor_stack: &mut ItemStack, inventory_stack: &mut ItemStack, is_output_slot: bool, interaction: InventoryInteraction) {
    if is_output_slot {
        if cursor_stack.item == inventory_stack.item {
            *cursor_stack += inventory_stack.count;
            *inventory_stack = ItemStack::empty();
        } else if !inventory_stack.is_empty() && cursor_stack.is_empty() {
            std::mem::swap(cursor_stack, inventory_stack);
        }
    } else {
        if cursor_stack.item == inventory_stack.item {
            match interaction {
                InventoryInteraction::TakeOne => {
                    *cursor_stack -= 1;
                    *inventory_stack += 1;
                },
                InventoryInteraction::TakeStack => {
                    *inventory_stack += cursor_stack.count;
                    *cursor_stack = ItemStack::empty();
                }
            };
        } else if !cursor_stack.is_empty() && inventory_stack.is_empty() {
            match interaction {
                InventoryInteraction::TakeOne => {
                    *inventory_stack = ItemStack::new(cursor_stack.item.clone(), 1);
                    *cursor_stack -= 1;
                },
                InventoryInteraction::TakeStack => {
                    std::mem::swap(cursor_stack, inventory_stack);
                }
            };
        } else if !inventory_stack.is_empty() && cursor_stack.is_empty() {
            match interaction {
                InventoryInteraction::TakeOne => {
                    *cursor_stack = ItemStack::new(inventory_stack.item.clone(), 1);
                    *inventory_stack -= 1;
                },
                InventoryInteraction::TakeStack => {
                    std::mem::swap(cursor_stack, inventory_stack);
                }
            };
        } else {
            std::mem::swap(cursor_stack, inventory_stack);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
#[derive(serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct ItemStack {
    pub item: ItemIDRep,
    pub count: i32,
}

impl ItemStack {
    pub fn new(item: ItemIDRep, count: i32) -> Self {
        Self {
            count,
            item,
        }
    }

    pub fn empty() -> Self {
        Self {
            item: items::NOTHING.into(),
            count: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count <= 0 || self.item == items::NOTHING
    }
}


impl AddAssign<i32> for ItemStack {
    fn add_assign(&mut self, rhs: i32) {
        self.count += rhs;
        if self.is_empty() {
            *self = Self::empty();
        }
    }
}

impl SubAssign<i32> for ItemStack {
    fn sub_assign(&mut self, rhs: i32) {
        self.add_assign(-rhs);
    }
}

pub enum InventoryInteraction {
    TakeStack,
    TakeOne,
}