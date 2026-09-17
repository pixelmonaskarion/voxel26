use std::{ops::AddAssign, time::Duration};

use glam::IVec3;

use crate::{blocks::KILN, chunk::ChunkManager, crafting::KILN_CRAFTING_TYPE, inventory::Inventory, registries::{BlockStateRegistry, Registries}};

pub trait BlockStateProvider: Sync {
    fn does_updates(&self) -> bool;
    fn update(&self, state: &mut Vec<u8>, world_position: IVec3, world: &mut ChunkManager, registries: &Registries, delta_time: Duration);
    fn new_state(&self, world_position: IVec3, world: &mut ChunkManager) -> Vec<u8>;
}

pub trait BlockState: Sync {
    type BlockState: serde::Serialize + serde::de::DeserializeOwned;
    fn does_updates(&self) -> bool;
    fn update(&self, state: &mut Self::BlockState, world_position: IVec3, world: &mut ChunkManager, registries: &Registries, delta_time: Duration);
    fn serialize(&self, state: Self::BlockState) -> Vec<u8> {
        serde_json::to_vec(&state).unwrap()
    }
    fn deserialize(&self, bytes: Vec<u8>) -> Self::BlockState {
        serde_json::from_slice(&bytes).unwrap()
    }
    fn new_state(&self, world_position: IVec3, world: &mut ChunkManager) -> Self::BlockState;
}

impl <T: BlockState> BlockStateProvider for T {
    fn does_updates(&self) -> bool {
        self.does_updates()
    }

    fn update(&self, state: &mut Vec<u8>, world_position: IVec3, world: &mut ChunkManager, registries: &Registries, delta_time: Duration) {
        let mut typed_state = self.deserialize(state.clone());
        self.update(&mut typed_state, world_position, world, registries, delta_time);
        *state = self.serialize(typed_state);
    }

    fn new_state(&self, world_position: IVec3, world: &mut ChunkManager) -> Vec<u8> {
        self.serialize(self.new_state(world_position, world))
    }
}

pub struct KilnBlockStateProvider;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct KilnBlockState {
    pub inventory: Inventory,
    pub cooking_time: Option<Duration>,
}

impl BlockState for KilnBlockStateProvider {
    type BlockState = KilnBlockState;
    fn does_updates(&self) -> bool {
        true
    }

    fn update(&self, state: &mut KilnBlockState, _world_position: IVec3, _world: &mut ChunkManager, registries: &Registries, delta_time: Duration) {
        let recipe_duration = Duration::from_secs_f32(3.0);
        if let Some(cooking_time) = &mut state.cooking_time && !state.inventory.items[0].is_empty() {
            if let Some(recipe) = registries.crafting_registry.get_recipe_for_pattern(vec![vec![state.inventory.items[0].clone()]], KILN_CRAFTING_TYPE, registries) && (state.inventory.items[1].item == recipe.result.item || state.inventory.items[1].is_empty()){
                cooking_time.add_assign(delta_time);
                if *cooking_time > recipe_duration {
                    state.cooking_time = None;
                    state.inventory.items[0] -= 1;
                    if state.inventory.items[1].is_empty() {
                        state.inventory.items[1] = recipe.result;
                    } else {
                        state.inventory.items[1] += recipe.result.count;
                    }
                }
            } else {
                state.cooking_time = None;
            }
        }
        if !state.inventory.items[0].is_empty() {
            if state.cooking_time.is_none() {
                if let Some(_) = registries.crafting_registry.get_recipe_for_pattern(vec![vec![state.inventory.items[0].clone()]], KILN_CRAFTING_TYPE, registries) {
                    state.cooking_time = Some(Duration::from_secs_f32(0.0));
                }
            }
        } else {
            state.cooking_time = None;
        }
    }

    fn new_state(&self, _world_position: IVec3, _world: &mut ChunkManager) -> KilnBlockState {
        KilnBlockState {
            inventory: Inventory::empty_size(2),
            cooking_time: None,
        }
    }
}

const KILN_STATE_PROVIDER: &KilnBlockStateProvider = &KilnBlockStateProvider;

impl BlockStateRegistry {
    pub fn register_all(&mut self) {
        self.register(KILN, KILN_STATE_PROVIDER);
    }
}