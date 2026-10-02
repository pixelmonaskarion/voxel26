use wgpu::{Buffer, BufferUsages, Device, util::{BufferInitDescriptor, DeviceExt}};

use crate::chunk::{CHUNK_SIZE, index_in_chunk};

#[derive(serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[derive(Clone)]
pub struct BlockLightingData(Vec<u8>);

// pub type SkyLightingData = LightingData<1>;

const COLORS_PER_BYTE: u32 = 2;

impl Default for BlockLightingData {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockLightingData {
    const COLORS_PER_BLOCK: u32 = 3;
    pub fn new() -> Self {
        
        Self(vec![0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * Self::COLORS_PER_BLOCK / COLORS_PER_BYTE) as usize])
    }

    pub fn get_color(&self, x: u32, y: u32, z: u32) -> [u8; 3] {
        let block_index = index_in_chunk(x, y, z);
        let light_index = block_index * Self::COLORS_PER_BLOCK as usize / COLORS_PER_BYTE as usize;
        let light_remainder = block_index * Self::COLORS_PER_BLOCK as usize % COLORS_PER_BYTE as usize;
        if light_remainder == 0 {
            return [
                self.0[light_index] >> 4,
                self.0[light_index] & 0x0F,
                self.0[light_index + 1] >> 4,
            ];
        } else {
            return [
                self.0[light_index] & 0x0F,
                self.0[light_index + 1] >> 4,
                self.0[light_index + 1] & 0x0F,
            ];
        }
    }

    pub fn set_color(&mut self, x: u32, y: u32, z: u32, light: [u8; Self::COLORS_PER_BLOCK as usize]) {
        let block_index = index_in_chunk(x, y, z);
        let light_index = block_index * Self::COLORS_PER_BLOCK as usize / COLORS_PER_BYTE as usize;
        let light_remainder = block_index * Self::COLORS_PER_BLOCK as usize % COLORS_PER_BYTE as usize;
        if light_remainder == 0 {
            self.0[light_index] = (light[0] << 4 & 0xF0) | (light[1] & 0x0F);
            self.0[light_index+1] = (light[2] << 4 & 0xF0) | (self.0[light_index+1] & 0x0F);
        } else {
            self.0[light_index] = (self.0[light_index] & 0xF0) | (light[0] & 0x0F);
            self.0[light_index+1] = (light[1] << 4 & 0xF0) | (light[2] & 0x0F);
        }
    }

    pub fn make_lighting_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            contents: &self.0,
            label: Some("Chunk Lighting Buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::STORAGE,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[derive(Clone)]
pub struct SkyLightingData(Vec<u8>);

impl Default for SkyLightingData {
    fn default() -> Self {
        Self::new()
    }
}

impl SkyLightingData {
    pub fn new() -> Self {
        
        Self(vec![0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE / COLORS_PER_BYTE) as usize])
    }

    pub fn get_color(&self, x: u32, y: u32, z: u32) -> u8  {
        let block_index = index_in_chunk(x, y, z);
        let light_index = block_index / COLORS_PER_BYTE as usize;
        let light_remainder = block_index % COLORS_PER_BYTE as usize;
        if light_remainder == 0 {
            return self.0[light_index] >> 4;
        } else {
            return self.0[light_index] & 0x0F;
        }
    }

    pub fn set_color(&mut self, x: u32, y: u32, z: u32, light: u8) {
        let block_index = index_in_chunk(x, y, z);
        let light_index = block_index / COLORS_PER_BYTE as usize;
        let light_remainder = block_index % COLORS_PER_BYTE as usize;
        if light_remainder == 0 {
            self.0[light_index] = (self.0[light_index] & 0x0F) | light << 4;
        } else {
            self.0[light_index] = (self.0[light_index] & 0xF0) | light;
        }
    }

    pub fn make_lighting_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            contents: &self.0,
            label: Some("Chunk Lighting Buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::STORAGE,
        })
    }
}

// #[derive(serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[derive(Clone)]
pub struct TopBlockData {
    data: Vec<i32>,
    // dirty: bool,
    // buffer: Buffer,
    // pub binding: BindGroup,
}

impl TopBlockData {
    pub fn new(device: &Device) -> Self {
        let data = vec![i32::MIN; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        // let buffer = device.create_buffer_init(&BufferInitDescriptor {
        //     contents: cast_slice(&data),
        //     label: None,
        //     usage: BufferUsages::STORAGE | BufferUsages::UNIFORM,
        // });
        // let binding = device.create_bind_group(&BindGroupDescriptor {
        //     label: None,
        //     layout: &ChunkManager::top_block_bind_group_layout(device),
        //     entries: &[
        //         BindGroupEntry {
        //             binding: 0,
        //             resource: buffer.as_entire_binding(),
        //         }
        //     ]
        // });
        Self {
            data,
            // dirty: false,
            // buffer,
            // binding
        }
    }

    pub fn get_top_block(&self, x: u32, z: u32) -> i32 {
        let block_index = x * CHUNK_SIZE + z;
        return self.data[block_index as usize];
    }

    pub fn set_top_block(&mut self, x: u32, z: u32, y: i32) {
        let block_index = x * CHUNK_SIZE + z;
        self.data[block_index as usize] = y;
        // self.dirty = true;
    }

    // pub fn update_buffer(&mut self, device: &Device) {
    //     if self.dirty {
    //         let buffer = device.create_buffer_init(&BufferInitDescriptor {
    //             contents: cast_slice(&self.data),
    //             label: None,
    //             usage: BufferUsages::STORAGE | BufferUsages::UNIFORM,
    //         });
    //         let binding = device.create_bind_group(&BindGroupDescriptor {
    //             label: None,
    //             layout: &ChunkManager::top_block_bind_group_layout(device),
    //             entries: &[
    //                 BindGroupEntry {
    //                     binding: 0,
    //                     resource: buffer.as_entire_binding(),
    //                 }
    //             ]
    //         });
    //         self.buffer = buffer;
    //         self.binding = binding;
    //         self.dirty = false;
    //     }
    // }
}

#[cfg(test)]
mod lighting_test {
    use crate::lighting::{BlockLightingData, SkyLightingData};

    #[test]
    fn test_lighting() {
        let mut lighting = BlockLightingData::new();
        lighting.set_color(0, 0, 0, [1, 2, 3]);
        lighting.set_color(0, 0, 1, [4, 5, 6]);
        assert_eq!([1, 2, 3], lighting.get_color(0, 0, 0));
        assert_eq!([4, 5, 6], lighting.get_color(0, 0, 1));

        let mut lighting = SkyLightingData::new();
        lighting.set_color(0, 0, 0, 1);
        lighting.set_color(0, 0, 1, 3);
        assert_eq!(1, lighting.get_color(0, 0, 0));
        assert_eq!(3, lighting.get_color(0, 0, 1));
    }
}