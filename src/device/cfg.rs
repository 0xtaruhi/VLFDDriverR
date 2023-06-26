pub struct Cfg {
    pub cfg: [u16; 64],
}

impl Cfg {
    pub fn new() -> Self {
        Self { cfg: [0u16; 64] }
    }

    pub fn fifo_size(&self) -> u16 {
        self.cfg[33]
    }

    pub fn is_programmed(&self) -> bool {
        self.cfg[48] & 0x0001 != 0
    }
}
