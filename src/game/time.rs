#[derive(Debug, Clone, Copy, Hash)]
pub struct Frame {
    frame_with_freeze: usize,
    frame_without_freeze: usize,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            frame_with_freeze: 0,
            frame_without_freeze: 0,
        }
    }

    pub fn tick(&mut self, frozen: bool) {
        self.frame_with_freeze += 1;
        if !frozen {
            self.frame_without_freeze += 1;
        }
    }

    pub fn since_without_freeze(self, frame: Frame) -> usize {
        self.frame_without_freeze - frame.frame_without_freeze
    }

    pub fn since_with_freeze(self, frame: Frame) -> usize {
        self.frame_with_freeze - frame.frame_with_freeze
    }
}
