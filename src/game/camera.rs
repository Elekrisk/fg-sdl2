use std::hash::Hash;

use super::{fixed_point::FixedPoint, super::fvec2::FVec2};

#[derive(Clone)]
pub struct Camera {
    pub center: FVec2,
    pub scale: f32,
    pub width: f32,
    pub height: f32,
    pub offset: FVec2,
}

impl Hash for Camera {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.center.x.to_bits().hash(state);
        self.center.y.to_bits().hash(state);
        self.scale.to_bits().hash(state);
        self.width.to_bits().hash(state);
        self.height.to_bits().hash(state);
    }
}

impl Camera {
    pub fn to_screen_space(&self, world_pos: impl Into<FVec2>) -> FVec2 {
        let q = world_pos.into() - self.center;
        let mut q = q * self.scale;
        let q = q + FVec2::new(self.width, self.height) / 2.0.into();

        q + self.offset
    }
}
