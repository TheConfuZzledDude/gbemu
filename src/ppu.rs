use core::{
    mem,
    ops::{BitAnd, BitOr, Deref, DerefMut, Index, IndexMut, Not, Shl, Shr},
};
use std::process::Output;

use crate::context::{Context, Memory8K};
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
    fn enable(&self) -> bool {
        get_bit(self.0, 7)
    }

    fn set_enable(&mut self, value: bool) {
        set_bit(&mut self.0, 7, value);
    }

    fn window_tile_map(&self) -> TileMapArea {
        TileMapArea::from_repr(get_bit(self.0, 6) as u8).unwrap()
    }

    fn set_window_tile_map(&mut self, map: TileMapArea) {
        set_bit(&mut self.0, 6, map as u8 != 0);
    }

    fn window_enable(&self) -> bool {
        get_bit(self.0, 5)
    }

    fn set_window_enable(&mut self, value: bool) {
        set_bit(&mut self.0, 5, value);
    }

    fn tile_data_area(&self) -> TileDataArea {
        TileDataArea::from_repr(get_bit(self.0, 4) as u8).unwrap()
    }

    fn set_tile_data_area(&mut self, map: TileDataArea) {
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

    fn obj_enable(&self) -> bool {
        get_bit(self.0, 1)
    }

    fn set_obj_enable(&mut self, value: bool) {
        set_bit(&mut self.0, 1, value);
    }
    fn bg_window_enable(&self) -> bool {
        get_bit(self.0, 0)
    }

    fn set_bg_window_enable(&mut self, value: bool) {
        set_bit(&mut self.0, 0, value);
    }
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
enum TileDataArea {
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
}
#[repr(transparent)]
#[derive(Clone, Copy)]
struct Oam([u8; 0xFEA0 - 0xFE00]);

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
struct OamAttribute(u8);

impl DerefMut for OamAttribute {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for OamAttribute {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OamAttribute {
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
    fn y(&self) -> &u8 {
        &self.0[0]
    }
    fn y_mut(&mut self) -> &mut u8 {
        &mut self.0[0]
    }
    fn x(&self) -> &u8 {
        &self.0[1]
    }
    fn x_mut(&mut self) -> &mut u8 {
        &mut self.0[1]
    }

    fn tile_index(&self) -> &u8 {
        &self.0[2]
    }
    fn tile_index_mut(&mut self) -> &mut u8 {
        &mut self.0[2]
    }

    fn attributes(&self) -> &OamAttribute {
        OamAttribute::wrap_ref(&self.0[3])
    }
    fn attributes_mut(&mut self) -> &mut OamAttribute {
        OamAttribute::wrap_mut(&mut self.0[3])
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
    Zero,
    One,
    Two,
    Three,
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
    current_mode: Mode,
}
