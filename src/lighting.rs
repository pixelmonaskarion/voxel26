use crate::chunk::{CHUNK_SIZE, index_in_chunk};

#[derive(serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[derive(Clone)]
pub struct LightingData(Vec<u8>);

const COLORS_PER_BYTE: u32 = 2;
const COLORS_PER_BLOCK: u32 = 3;

impl LightingData {
    pub fn new() -> Self {
        
        Self(vec![0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * COLORS_PER_BLOCK / COLORS_PER_BYTE) as usize])
    }

    pub fn get_color(&self, x: u32, y: u32, z: u32) -> [u8; 3] {
        let block_index = index_in_chunk(x, y, z);
        let light_index = block_index * COLORS_PER_BLOCK as usize / COLORS_PER_BYTE as usize;
        let light_remainder = block_index * COLORS_PER_BLOCK as usize % COLORS_PER_BYTE as usize;
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

    pub fn set_color(&mut self, x: u32, y: u32, z: u32, light: [u8; 3]) {
        let block_index = index_in_chunk(x, y, z);
        let light_index = block_index * COLORS_PER_BLOCK as usize / COLORS_PER_BYTE as usize;
        let light_remainder = block_index * COLORS_PER_BLOCK as usize % COLORS_PER_BYTE as usize;
        if light_remainder == 0 {
            self.0[light_index] = (light[0] << 4 & 0xF0) | (light[1] & 0x0F);
            self.0[light_index+1] = (light[2] << 4 & 0xF0) | (self.0[light_index+1] & 0x0F);
        } else {
            self.0[light_index] = (self.0[light_index] & 0xF0) | (light[0] & 0x0F);
            self.0[light_index+1] = (light[1] << 4 & 0xF0) | (light[2] & 0x0F);
        }
    }
}

#[cfg(test)]
mod lighting_test {
    use crate::lighting::LightingData;

    #[test]
    fn test_lighting() {
        let mut lighting = LightingData::new();
        lighting.set_color(0, 0, 0, [1, 2, 3]);
        lighting.set_color(0, 0, 1, [4, 5, 6]);
        assert_eq!([1, 2, 3], lighting.get_color(0, 0, 0));
        assert_eq!([4, 5, 6], lighting.get_color(0, 0, 1));
    }
}