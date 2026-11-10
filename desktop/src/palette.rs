#[derive(Clone)]
pub struct ColorPalette {
    pub name: String,
    pub colors: Vec<[u8; 4]>,
}

impl Default for ColorPalette {
    fn default() -> Self {
        builtin_palettes()[0].clone()
    }
}

impl ColorPalette {
    pub fn new(name: impl Into<String>, colors: Vec<[u8; 4]>) -> Self {
        Self {
            name: name.into(),
            colors,
        }
    }
}

const fn hex(hex: u32) -> [u8; 4] {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]
}

pub fn builtin_palettes() -> Vec<ColorPalette> {
    vec![
        ColorPalette::new(
            "Grayscale",
            vec![hex(0xFFFFFF), hex(0xA0A4A0), hex(0x686C68), hex(0x000000)],
        ),
        ColorPalette::new(
            "ICE CREAM GB",
            vec![hex(0xFFF6D3), hex(0xF9A875), hex(0xEBCB6F), hex(0x7C3F58)],
        ),
        ColorPalette::new(
            "VELVET CHERRY GB PALETTE",
            vec![hex(0x2d162c), hex(0x412752), hex(0x683a68), hex(0x9775a6)],
        ),
        ColorPalette::new(
            "NYMPH GB PALETTE",
            vec![hex(0x2c2137), hex(0x446176), hex(0x3fac95), hex(0xa1ef8c)],
        ),
    ]
}
