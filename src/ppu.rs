#![allow(unused)]

use core::{
    iter::Cloned,
    mem,
    ops::{BitAnd, BitOr, Deref, DerefMut, Index, IndexMut, Not, Shl, Shr},
    slice,
};
use std::process::Output;

use crate::context::{Context, Memory8K};
use array_deque::{ArrayDeque, StackArrayDeque};
use better_default::Default;
use bytemuck::TransparentWrapper;
use paste::paste;
use strum::FromRepr;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Lcdc(u8);

fn set_bit<T>(num: &mut T, index: u8, value: bool)
where
    T: BitAnd<T, Output = T> + BitOr<T, Output = T>,
    T: From<bool> + Copy,
    T: Shl<u8, Output = T>,
    T: Not<Output = T>,
{
    *num = (*num & !(T::from(true) << index)) | (T::from(value) << index);
}
fn get_bit<T>(num: T, index: u8) -> bool
where
    T: BitAnd<T, Output = T> + BitOr<T, Output = T>,
    T: From<bool> + Copy,
    T: Shr<u8, Output = T>,
    T: Not<Output = T>,
    T: PartialEq,
{
    (num >> index) & T::from(true) == T::from(true)
}

macro_rules! bit_getters {
    ($name:ident,$bit:literal) => {
        fn $name(&self) -> bool {
            get_bit(self.0, $bit)
        }

        paste! {
            fn [<set_ $name>](&mut self, value: bool) {
                set_bit(&mut self.0, $bit, value);
            }
        }
    };
}

impl DerefMut for Lcdc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Lcdc {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Lcdc {
    bit_getters!(enable, 7);

    fn window_tile_map(&self) -> TileMapArea {
        TileMapArea::from_repr(get_bit(self.0, 6) as u8).unwrap()
    }

    fn set_window_tile_map(&mut self, map: TileMapArea) {
        set_bit(&mut self.0, 6, map as u8 != 0);
    }

    bit_getters!(window_enable, 5);

    fn tile_data_mapping(&self) -> TileDataMapping {
        TileDataMapping::from_repr(get_bit(self.0, 4) as u8).unwrap()
    }

    fn set_tile_data_mapping(&mut self, map: TileDataMapping) {
        set_bit(&mut self.0, 4, map as u8 != 0);
    }

    fn bg_tile_map(&self) -> TileMapArea {
        TileMapArea::from_repr(get_bit(self.0, 3) as u8).unwrap()
    }

    fn set_bg_tile_map(&mut self, map: TileMapArea) {
        set_bit(&mut self.0, 3, map as u8 != 0);
    }

    fn obj_size(&self) -> ObjSize {
        ObjSize::from_repr(get_bit(self.0, 2) as u8).unwrap()
    }

    fn set_obj_size(&mut self, map: ObjSize) {
        set_bit(&mut self.0, 2, map as u8 != 0);
    }

    bit_getters!(obj_enable, 1);

    bit_getters!(bg_window_enable, 0);
}

#[derive(Debug, Default)]
pub(crate) struct LCDRegisters {
    lcdc: Lcdc,
    stat: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    bgp: u8,
    obp: [u8; 2],
    wy: u8,
    wx: u8,
}

#[repr(transparent)]
#[derive(Default)]
pub(crate) struct Vram(#[default([0; 1024 * 8])] Memory8K);

impl DerefMut for Vram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Vram {
    type Target = Memory8K;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Debug, FromRepr)]
#[repr(u8)]
enum TileMapArea {
    Zero,
    One,
}

#[derive(Copy, Clone, Debug, FromRepr)]
#[repr(u8)]
enum ObjSize {
    Square,
    Tall,
}

#[derive(Copy, Clone, Debug, FromRepr)]
#[repr(u8)]
enum TileDataMapping {
    Zero,
    One,
}

const VRAM_BASE_ADDRESS: usize = 0x8000;

impl Vram {
    fn tile_map(&self, index: TileMapArea) -> &[u8] {
        &self.0[Self::get_tile_map_range(index)]
    }
    fn tile_map_mut(&mut self, index: TileMapArea) -> &mut [u8] {
        &mut self.0[Self::get_tile_map_range(index)]
    }

    fn get_tile_map_range(index: TileMapArea) -> std::ops::RangeInclusive<usize> {
        match index {
            TileMapArea::Zero => 0x9800 - VRAM_BASE_ADDRESS..=0x9BFF - VRAM_BASE_ADDRESS,
            TileMapArea::One => 0x9C00 - VRAM_BASE_ADDRESS..=0x9FFF - VRAM_BASE_ADDRESS,
        }
    }

    fn window_tile_map(&self, ctx: &Context) -> &[u8] {
        self.tile_map(ctx.memory.io.lcd.lcdc.window_tile_map())
    }
    fn window_tile_map_mut(&mut self, ctx: &mut Context) -> &mut [u8] {
        self.tile_map_mut(ctx.memory.io.lcd.lcdc.window_tile_map())
    }

    fn bg_tile_data(&self, mapping: TileDataMapping, tile_no: u8) -> &[u8; 16] {
        match mapping {
            TileDataMapping::Zero => {
                let tile_data = &self.0[0x8000 - VRAM_BASE_ADDRESS..=0x8FFF - VRAM_BASE_ADDRESS];
                let (tiles, []) = tile_data.as_chunks::<16>() else {
                    unreachable!()
                };
                &tiles[tile_no as usize]
            }
            TileDataMapping::One => {
                let tile_data = &self.0[0x8800 - VRAM_BASE_ADDRESS..=0x97FF - VRAM_BASE_ADDRESS];
                let (tiles, []) = tile_data.as_chunks::<16>() else {
                    unreachable!()
                };
                &tiles[tile_no.wrapping_add(128) as usize]
            }
        }
    }
    fn bg_tile_data_mut(&mut self, mapping: TileDataMapping, tile_no: u8) -> &mut [u8; 16] {
        match mapping {
            TileDataMapping::Zero => {
                let tile_data =
                    &mut self.0[0x8000 - VRAM_BASE_ADDRESS..=0x8FFF - VRAM_BASE_ADDRESS];
                let (tiles, []) = tile_data.as_chunks_mut::<16>() else {
                    unreachable!()
                };
                &mut tiles[tile_no as usize]
            }
            TileDataMapping::One => {
                let tile_data =
                    &mut self.0[0x8800 - VRAM_BASE_ADDRESS..=0x97FF - VRAM_BASE_ADDRESS];
                let (tiles, []) = tile_data.as_chunks_mut::<16>() else {
                    unreachable!()
                };
                &mut tiles[tile_no.wrapping_add(128) as usize]
            }
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Copy, Default)]
pub(crate) struct Oam(#[default([0;0xFEA0 - 0xFE00])] [u8; 0xFEA0 - 0xFE00]);

impl DerefMut for Oam {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Oam {
    type Target = [u8; 0xFEA0 - 0xFE00];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Oam {
    fn oam_entries(&self) -> &[OamEntry] {
        let (oam_entries, []) = self.0.as_chunks::<4>() else {
            unreachable!()
        };
        OamEntry::wrap_slice(oam_entries)
    }

    fn oam_entries_mut(&mut self) -> &[OamEntry] {
        let (oam_entries, []) = self.0.as_chunks_mut::<4>() else {
            unreachable!()
        };
        OamEntry::wrap_slice_mut(oam_entries)
    }
}

#[derive(Debug, Clone, Copy, Default, TransparentWrapper)]
#[repr(transparent)]
struct OamAttributes(u8);

impl DerefMut for OamAttributes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for OamAttributes {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OamAttributes {
    bit_getters!(priority, 7);
    bit_getters!(y_flip, 6);
    bit_getters!(x_flip, 5);
    bit_getters!(dmg_palette, 4);
    bit_getters!(bank, 3);
    fn cgb_palette(&self) -> u8 {
        self.0 & 0b111
    }
    fn set_cgb_palette(&mut self, value: u8) {
        assert!(value <= 0b111);
        self.0 = (self.0 & !0b111) | (value & 0b111);
    }
}

#[derive(Debug, Clone, Copy, Default, TransparentWrapper)]
#[repr(transparent)]
struct OamEntry([u8; 4]);

impl DerefMut for OamEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Deref for OamEntry {
    type Target = [u8; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl OamEntry {
    fn y(&self) -> u8 {
        self.0[0]
    }
    fn y_mut(&mut self) -> &mut u8 {
        &mut self.0[0]
    }
    fn x(&self) -> u8 {
        self.0[1]
    }
    fn x_mut(&mut self) -> &mut u8 {
        &mut self.0[1]
    }

    fn tile_index(&self) -> u8 {
        self.0[2]
    }
    fn tile_index_mut(&mut self) -> &mut u8 {
        &mut self.0[2]
    }

    fn attributes(&self) -> &OamAttributes {
        OamAttributes::wrap_ref(&self.0[3])
    }
    fn attributes_mut(&mut self) -> &mut OamAttributes {
        OamAttributes::wrap_mut(&mut self.0[3])
    }
}

#[derive(Copy, Clone, Debug, FromRepr)]
enum Mode {
    OamScan = 2,
    PixelTransfer = 3,

    HBlank = 0,
    VBlank = 1,
}

struct FifoPixel {
    pixel: Pixel,
    source: PixelSource,
}

#[derive(Debug, Clone, Copy, FromRepr)]
#[repr(u8)]
enum Pixel {
    White,
    LightGray,
    DarkGrey,
    Black,
}

#[derive(Debug, Clone, Copy, FromRepr)]
enum PixelSource {
    BG = 10,
    S0 = 0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
}

// Reference for PPU details: https://www.youtube.com/watch?v=HyzD8pNlpwI&t=1760s
struct PPU {
    cycle_counter: u32,
    current_mode: Mode,
    obj_buffer: StackArrayDeque<OamEntry, 10>,
    oam_copy: ArrayDeque<OamEntry>,
    pixel_fifo: StackArrayDeque<FifoPixel, 16>,
    fifo_state: FifoState,
    screen_x: u8,
}

#[derive(Default, Clone, Copy, Debug)]
struct FifoState {
    tile_no: Option<u8>,
    data_low: Option<u8>,
    data_high: Option<u8>,
    buffer: [u8; 8],
    tile_line: u8,
}

impl PPU {
    fn tick(&mut self, ctx: &mut Context) {
        match self.current_mode {
            Mode::OamScan => self.oam_scan(ctx),
            Mode::PixelTransfer => self.pixel_transfer(ctx),
            Mode::HBlank => todo!(),
            Mode::VBlank => todo!(),
        };
        self.cycle_counter += 1;
    }

    fn oam_scan(&mut self, ctx: &mut Context) {
        match self.cycle_counter {
            0 => {
                self.obj_buffer.clear();
                self.oam_copy = ArrayDeque::from(ctx.memory.oam.oam_entries());
            }
            x if x % 2 == 0 => {
                //Stall
            }
            x => {
                if !self.obj_buffer.is_full()
                    && let Some(next_obj) = self.oam_copy.pop_front()
                    && next_obj.x() != 0
                    && object_on_scanline(
                        next_obj.y(),
                        ctx.memory.io.lcd.ly,
                        ctx.memory.io.lcd.lcdc.obj_size(),
                    )
                {
                    self.obj_buffer.push_back(next_obj);
                }
            }
        }
        if self.cycle_counter == 79 {
            self.current_mode = Mode::PixelTransfer
        }
    }

    fn pixel_transfer(&mut self, ctx: &mut Context) {
        if self.pixel_fifo.len() > 8 {
            let pixel = self.pixel_fifo.pop_front();
            // pusht he pixel
        }
        if self.cycle_counter % 2 == 1 {
            if self.fifo_state.tile_no.is_none() {
                self.fetch_tile(ctx);
            } else if self.fifo_state.data_low.is_none() {
                let Some(tile_no) = self.fifo_state.tile_no else {
                    unreachable!()
                };
                self.fifo_state.data_low = Some(
                    ctx.memory
                        .vram
                        .bg_tile_data(ctx.memory.io.lcd.lcdc.tile_data_mapping(), tile_no)
                        [self.fifo_state.tile_line as usize * 2],
                )
            } else if self.fifo_state.data_high.is_none() {
                let Some(tile_no) = self.fifo_state.tile_no else {
                    unreachable!()
                };
                self.fifo_state.data_high = Some(
                    ctx.memory
                        .vram
                        .bg_tile_data(ctx.memory.io.lcd.lcdc.tile_data_mapping(), tile_no)
                        [self.fifo_state.tile_line as usize * 2 + 1],
                )
            }
            if let (Some(low), Some(high)) = (self.fifo_state.data_low, self.fifo_state.data_high)
                && self.pixel_fifo.len() <= 8
            {
                let tile_row = u16::from_le_bytes([low, high]);
                for n in 0..8 {
                    let palette_index = tile_row.extract_bits(0b1000_0000_1000_0000 >> n);
                    let palette = ctx.memory.io.lcd.bgp;
                    let colour = Pixel::from_repr((palette >> (palette_index * 2)) & 0b11).unwrap();
                    self.pixel_fifo.push_back(FifoPixel {
                        pixel: colour,
                        source: PixelSource::BG,
                    });
                }
            }
        }
    }

    fn fetch_tile(&mut self, ctx: &mut Context) {
        let in_window = self.screen_x >= ctx.memory.io.lcd.wx
            && ctx.memory.io.lcd.ly >= ctx.memory.io.lcd.wy
            && ctx.memory.io.lcd.lcdc.window_enable();
        let tile_map_area = if !in_window {
            ctx.memory.io.lcd.lcdc.bg_tile_map()
        } else {
            ctx.memory.io.lcd.lcdc.window_tile_map()
        };
        let tile_x = if !in_window {
            self.screen_x.wrapping_add(ctx.memory.io.lcd.scx)
        } else {
            self.screen_x - ctx.memory.io.lcd.wx
        };
        let tile_y = if !in_window {
            ctx.memory.io.lcd.ly.wrapping_add(ctx.memory.io.lcd.scy)
        } else {
            ctx.memory.io.lcd.ly - ctx.memory.io.lcd.wy
        };
        self.fifo_state.tile_line = tile_y % 8;
        let tile_map_index = ((tile_y as u16) / 8) << 5 | (tile_x as u16 / 8);
        self.fifo_state.tile_no =
            Some(ctx.memory.vram.tile_map(tile_map_area)[tile_map_index as usize]);
    }
}

fn object_on_scanline(obj_y: u8, scanline_y: u8, size: ObjSize) -> bool {
    match size {
        ObjSize::Square => (obj_y..(obj_y + 8)),
        ObjSize::Tall => (obj_y..(obj_y + 16)),
    }
    .contains(&scanline_y)
}
