mod fetcher;
mod lcdc;
mod sprites;

use fetcher::{FetcherState, FetcherStep};
use sprites::Sprite;
use std::{collections::VecDeque, fs::File, io::Write, vec};

use crate::ppu::{lcdc::LCDC, sprites::OamPixel};

const RESOLUTION_X: usize = 160;
const RESOLUTION_Y: usize = 144;
const TRANSPARENT: OamPixel = OamPixel {
    color_idx: 0,
    palette: 0,
    priority: 0,
};

enum PpuMode {
    OamScan,
    Drawing,
    HBlank,
    VBlank,
}

pub struct Ppu {
    vram: [u8; 0x2000],
    oam: [u8; 0x00A0],

    lcdc: lcdc::LCDC,
    lcds: u8,

    scy: u8,
    scx: u8,
    lx: u8,
    ly: u8,
    lyc: u8,
    dma: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,

    dot: u16,
    mode: PpuMode,

    oambuffer: Vec<Sprite>,
    pixel_fifo: VecDeque<u8>,
    oam_fifo: VecDeque<OamPixel>,
    fetcher: FetcherState,
    wly: u8, // Invisible to the CPU

    interrupt_flags: u8,
    stat_line: bool,

    window_active: bool,
    scx_discard_counter: Option<u8>,

    framebuffer: [u8; (RESOLUTION_X * RESOLUTION_Y) as usize],
    pub drawn: [bool; (RESOLUTION_X * RESOLUTION_Y) as usize], // Debugging which pixels have been drawn
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            vram: [0; 0x2000],
            oam: [0; 0x00A0],
            lcdc: LCDC::new(),
            lcds: 0x85,
            scy: 0,
            scx: 0,
            lx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: 0xE4,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            dot: 0,
            mode: PpuMode::OamScan,
            oambuffer: vec![],
            pixel_fifo: VecDeque::new(),
            oam_fifo: VecDeque::new(),
            fetcher: FetcherState::new(),
            wly: 0,
            interrupt_flags: 0,
            stat_line: false,
            window_active: false,
            scx_discard_counter: None,
            framebuffer: [0; RESOLUTION_X * RESOLUTION_Y],
            drawn: [false; RESOLUTION_X * RESOLUTION_Y],
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        Ppu::default()
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => match self.mode {
                PpuMode::Drawing => 0xFF,
                _ => self.vram[(addr - 0x8000) as usize],
            },
            0xFE00..=0xFE9F => match self.mode {
                PpuMode::OamScan | PpuMode::Drawing => 0xFF,
                _ => self.oam[(addr - 0xFE00) as usize],
            },
            0xFF40 => self.lcdc.get(),
            0xFF41 => self.lcds | 0x80,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF46 => 0xFF, // DMA is write-only from the CPU's perspective
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF => match self.mode {
                PpuMode::Drawing => {}
                _ => self.vram[(addr - 0x8000) as usize] = value,
            },
            0xFE00..=0xFE9F => match self.mode {
                PpuMode::OamScan | PpuMode::Drawing => {}
                _ => self.oam[(addr - 0xFE00) as usize] = value,
            },
            0xFF40 => {
                let old = self.lcdc.get();
                if (old & 0x80) != (value & 0x80) {
                    if value & 0x80 == 0 {
                        self.disable_ppu();
                    } else {
                        self.enable_ppu();
                    }
                }
                self.lcdc.set(value)
            }
            0xFF41 => {
                self.lcds = (self.lcds & 0x07) | (value & 0x78);
            }
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => {}
            0xFF45 => self.lyc = value,
            0xFF46 => {
                self.dma = value;
                todo!("Implement DMA transfer from CPU to PPU OAM");
            }
            0xFF47 => self.bgp = value,
            0xFF48 => self.obp0 = value,
            0xFF49 => self.obp1 = value,
            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,
            _ => {}
        }
    }

    pub fn dump_vram(&self) -> std::io::Result<()> {
        let mut file = File::create("vramdump.log")?;
        let base_addr = 0x8000;

        for (i, chunk) in self.vram.chunks(16).enumerate() {
            write!(file, "{:04x}: ", base_addr + i * 16)?;

            for byte in chunk {
                write!(file, "{:02x} ", byte)?;
            }

            writeln!(file)?;
        }

        Ok(())
    }

    pub fn dump_oam(&self) -> std::io::Result<()> {
        let mut file = File::create("oamdump.log")?;
        let base_addr = 0xFE00;

        for (i, chunk) in self.oam.chunks(16).enumerate() {
            write!(file, "{:04x}: ", base_addr + i * 16)?;

            for byte in chunk {
                write!(file, "{:02x} ", byte)?;
            }

            writeln!(file)?;
        }

        Ok(())
    }

    pub fn oam_write_raw(&mut self, index: usize, value: u8) {
        if index < 160 {
            self.oam[index] = value;
        }
    }

    pub fn tick(&mut self) -> u8 {
        if !self.lcdc.is_ppu_enabled() {
            return 0;
        }
        let dot = self.dot;
        self.interrupt_flags = 0;

        match self.mode {
            PpuMode::OamScan => {
                if dot % 2 == 0 {
                    self.oam_scan();
                }
                if dot == 80 {
                    self.set_ppu_mode(PpuMode::Drawing);
                }
            }
            PpuMode::Drawing => {
                self.fetch_sprites(); // before the BG pixel for this lx is popped

                if !self.pixel_fifo.is_empty() {
                    if let Some(v) = self.scx_discard_counter {
                        self.pixel_fifo.pop_front();
                        self.scx_discard_counter = if v > 1 { Some(v - 1) } else { None };
                    } else if let Some(pixel) = self.pixel_fifo.pop_front() {
                        if self.should_start_window() {
                            self.window_active = true;
                            self.pixel_fifo.clear();
                            self.fetcher.restart_window();
                        } else {
                            let obj = self.pop_obj_pixel();
                            self.render_pixel(pixel, obj);
                        }
                    }
                }

                self.tick_fetcher();

                if self.lx == 160 {
                    self.set_ppu_mode(PpuMode::HBlank);
                }
            }
            PpuMode::HBlank => {}
            PpuMode::VBlank => {}
        }
        self.dot += 1;
        if self.dot == 456 {
            if self.window_active {
                self.wly += 1;
            }
            self.dot = 0;
            self.ly += 1;

            match self.ly {
                144 => {
                    self.set_ppu_mode(PpuMode::VBlank);
                }
                154 => {
                    self.ly = 0;
                    self.wly = 0;
                    self.window_active = false;
                    self.start_line();
                }
                1..=143 => self.start_line(),
                _ => {}
            }
            self.update_stat_line();
        }
        self.interrupt_flags
    }

    fn disable_ppu(&mut self) {
        self.ly = 0;
        self.lx = 0;
        self.dot = 0;
        self.set_ppu_mode(PpuMode::HBlank);
    }

    fn enable_ppu(&mut self) {
        self.set_ppu_mode(PpuMode::OamScan);
    }

    fn update_stat_line(&mut self) {
        let coincidence = self.ly == self.lyc;

        if coincidence {
            self.lcds |= 0x04;
        } else {
            self.lcds &= !0x04;
        }

        let mode = self.lcds & 0x03;

        let stat_line = (mode == 0 && self.lcds & 0x08 != 0)
            || (mode == 1 && self.lcds & 0x10 != 0)
            || (mode == 2 && self.lcds & 0x20 != 0)
            || (coincidence && self.lcds & 0x40 != 0);

        if stat_line && !self.stat_line {
            self.interrupt_flags |= 0x02;
        }

        self.stat_line = stat_line;
    }

    fn start_line(&mut self) {
        self.set_ppu_mode(PpuMode::OamScan);
        let scroll_discard_value = self.scx % 8;
        self.scx_discard_counter = if scroll_discard_value == 0 {
            None
        } else {
            Some(scroll_discard_value)
        };
        self.lx = 0;
        self.oambuffer.clear();
        self.pixel_fifo.clear();
        self.fetcher = FetcherState::new();
        self.oam_fifo.clear();
        self.oam_fifo.extend(std::iter::repeat(TRANSPARENT).take(8));
        self.window_active = false;
    }

    fn oam_scan(&mut self) {
        let sprite_index = (self.dot / 2) as usize;
        let offset = sprite_index * 4;

        if sprite_index >= 40 {
            return;
        }

        let sprite = Sprite::new(
            self.oam[offset + 1],
            self.oam[offset],
            self.oam[offset + 2],
            self.oam[offset + 3],
        );

        let sprite_height = self.lcdc.sprite_height();

        let y = self.ly as u16 + 16;
        if y >= sprite.y as u16
            && y < sprite.y as u16 + sprite_height as u16
            && self.oambuffer.len() < 10
        {
            self.oambuffer.push(sprite);
        }
    }

    fn tick_fetcher(&mut self) {
        self.fetcher.ticks += 1;

        if self.fetcher.ticks < 2 && self.fetcher.current_step() != FetcherStep::Push {
            return;
        }

        match self.fetcher.current_step() {
            FetcherStep::GetTile => {
                self.fetcher.set_tilemap_addr(
                    self.is_window_tile(),
                    self.lcdc.bg_tilemap_high(),
                    self.lcdc.window_tilemap_high(),
                );
                let addr = self.fetcher.get_tile_id_addr(
                    self.is_window_tile(),
                    self.scx,
                    self.scy,
                    self.ly,
                    self.wly,
                );
                let tile_id = self.vram[(addr - 0x8000) as usize];
                self.fetcher.set_tile_id(tile_id);
                self.fetcher.advance_step();
            }

            FetcherStep::GetTileDataLow => {
                let addr = self.fetcher.tile_data_addr(
                    self.lcdc.is_tile_data_mode1(),
                    self.is_window_tile(),
                    self.wly,
                    self.ly,
                    self.scy,
                );
                let low = self.vram[(addr - 0x8000) as usize];
                self.fetcher.set_tile_data_low(low);
                self.fetcher.advance_step();
            }

            FetcherStep::GetTileDataHigh => {
                let addr = self.fetcher.tile_data_addr(
                    self.lcdc.is_tile_data_mode1(),
                    self.is_window_tile(),
                    self.wly,
                    self.ly,
                    self.scy,
                );
                let high = self.vram[(addr + 1 - 0x8000) as usize];
                self.fetcher.set_tile_data_high(high);
                self.fetcher.advance_step();
            }

            FetcherStep::Sleep => {
                self.fetcher.advance_step();
            }

            FetcherStep::Push => {
                self.fetcher.push_pixels(&mut self.pixel_fifo);
            }
        }
    }

    fn fetch_sprites(&mut self) {
        if !self.lcdc.is_sprites_enabled() || self.oambuffer.is_empty() {
            return;
        }
        // stable sort keeps OAM order on ties, so lower X (then lower index) wins
        self.oambuffer.sort_by_key(|s| s.x);

        let lx = self.lx;
        let due: Vec<Sprite> = self.oambuffer.extract_if(.., |s| s.x <= lx + 8).collect();

        let height = self.lcdc.sprite_height();

        for sprite in due {
            // how many left-hand pixels are off screen (only when x < 8, at lx == 0)
            let skip = 8u8.saturating_sub(sprite.x) as usize;

            let line = (self.ly + 16).wrapping_sub(sprite.y);
            let row = if sprite.y_flip() {
                height - 1 - line
            } else {
                line
            };

            let tile = if height == 16 {
                (sprite.tile_idx & 0xFE) | (row >= 8) as u8
            } else {
                sprite.tile_idx
            };

            let addr = tile as usize * 16 + (row % 8) as usize * 2;
            let lo = self.vram[addr];
            let hi = self.vram[addr + 1];

            for i in skip..8 {
                let bit = if sprite.x_flip() { i } else { 7 - i };
                let color_idx = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
                if color_idx == 0 {
                    continue; // transparent
                }
                let slot = &mut self.oam_fifo[i - skip];
                if slot.color_idx == 0 {
                    *slot = OamPixel {
                        color_idx,
                        palette: sprite.palette(),
                        priority: sprite.priority(),
                    };
                }
            }
        }
    }

    fn pop_obj_pixel(&mut self) -> OamPixel {
        self.oam_fifo.push_back(TRANSPARENT);
        self.oam_fifo.pop_front().unwrap_or(TRANSPARENT)
    }

    fn render_pixel(&mut self, bg: u8, obj: OamPixel) {
        let bg = if self.lcdc.is_bg_enabled() { bg } else { 0 }; // LCDC bit 0

        let shade = if obj.color_idx != 0 && !(obj.priority != 0 && bg != 0) {
            let pal = if obj.palette == 0 {
                self.obp0
            } else {
                self.obp1
            };
            (pal >> (obj.color_idx * 2)) & 0b11
        } else {
            (self.bgp >> (bg * 2)) & 0b11
        };

        let i = self.ly as usize * 160 + self.lx as usize;
        self.framebuffer[i] = shade;
        self.drawn[i] = true;
        self.lx += 1;
    }

    fn in_window(&self) -> bool {
        (self.lx.wrapping_add(7)) >= self.wx
    }

    fn is_window_tile(&self) -> bool {
        return self.lcdc.is_window_enabled() && self.in_window() && self.ly >= self.wy;
    }

    fn should_start_window(&self) -> bool {
        !self.window_active && self.is_window_tile()
    }

    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }

    fn set_ppu_mode(&mut self, mode: PpuMode) {
        self.mode = mode;

        self.lcds &= !0b11;

        self.lcds |= match self.mode {
            PpuMode::HBlank => 0b00,
            PpuMode::VBlank => {
                self.interrupt_flags |= 0b0000_0001;
                0b01
            }
            PpuMode::OamScan => 0b10,
            PpuMode::Drawing => 0b11,
        };
    }
}
