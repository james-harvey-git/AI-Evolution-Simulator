use macroquad::prelude::*;

pub struct World {
    pub width: f32,
    pub height: f32,
    pub toroidal: bool,
}

impl World {
    pub fn new(width: f32, height: f32, toroidal: bool) -> Self {
        Self {
            width,
            height,
            toroidal,
        }
    }

    pub fn center(&self) -> Vec2 {
        vec2(self.width * 0.5, self.height * 0.5)
    }

    /// Wrap position into world bounds (toroidal).
    pub fn wrap(&self, mut pos: Vec2) -> Vec2 {
        if !self.toroidal {
            pos.x = pos.x.clamp(0.0, self.width);
            pos.y = pos.y.clamp(0.0, self.height);
            return pos;
        }
        pos.x = pos.x.rem_euclid(self.width);
        pos.y = pos.y.rem_euclid(self.height);
        pos
    }

    /// Shortest displacement vector from `from` to `to`, accounting for wrapping.
    pub fn delta(&self, from: Vec2, to: Vec2) -> Vec2 {
        let mut d = to - from;
        if self.toroidal {
            let hw = self.width * 0.5;
            let hh = self.height * 0.5;
            if d.x > hw {
                d.x -= self.width;
            } else if d.x < -hw {
                d.x += self.width;
            }
            if d.y > hh {
                d.y -= self.height;
            } else if d.y < -hh {
                d.y += self.height;
            }
        }
        d
    }

    /// Squared distance using shortest path.
    pub fn distance_sq(&self, a: Vec2, b: Vec2) -> f32 {
        self.delta(a, b).length_squared()
    }

    /// Distance using shortest path.
    pub fn distance(&self, a: Vec2, b: Vec2) -> f32 {
        self.delta(a, b).length()
    }
}
