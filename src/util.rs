pub struct RingIter {
    max: i32,
    d: i32,
    x: i32,
    y: i32,
    z_sign: bool,
    finished: bool,
}

pub fn positions(max_range: i32) -> RingIter {
    RingIter {
        max: max_range,
        d: 0,
        x: 0,
        y: 0,
        z_sign: false,
        finished: false,
    }
}

impl Iterator for RingIter {
    type Item = [i32; 3];

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            let rem1 = self.d - self.x.abs();
            let rem2 = rem1 - self.y.abs();

            let z = if self.z_sign { -rem2 } else { rem2 };
            let out = [self.x, self.y, z];

            if rem2 > 0 && !self.z_sign {
                self.z_sign = true;
                return Some(out);
            }

            self.z_sign = false;

            self.y += 1;
            if self.y > rem1 {
                self.x += 1;
                if self.x > self.d {
                    self.d += 1;
                    if self.d > self.max {
                        self.finished = true;
                        return None;
                    }
                    self.x = -self.d;
                }
                let rem1 = self.d - self.x.abs();
                self.y = -rem1;
            }

            return Some(out);
        }
    }
}

pub fn neighbors(chunk_pos: [i32; 3]) -> [[i32; 3]; 6] {
    [
        [chunk_pos[0]+1, chunk_pos[1], chunk_pos[2]],
        [chunk_pos[0]-1, chunk_pos[1], chunk_pos[2]],
        [chunk_pos[0], chunk_pos[1]+1, chunk_pos[2]],
        [chunk_pos[0], chunk_pos[1]-1, chunk_pos[2]],
        [chunk_pos[0], chunk_pos[1], chunk_pos[2]+1],
        [chunk_pos[0], chunk_pos[1], chunk_pos[2]-1],
    ]
}