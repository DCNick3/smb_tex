use std::fmt::Debug;
use std::io::{Read, Seek};
use anyhow::Result;
use binrw::{BinRead, BinReaderExt, BinResult, BinrwNamedArgs, BinWrite, FilePtr32, ReadOptions};
use image::{DynamicImage, Rgb, Rgba, RgbaImage, RgbImage};

#[derive(BinRead, Debug)]
pub struct TexturePackage {
    pub texture_count: u32,
    #[br(count = texture_count)]
    pub textures: FilePtr32<Vec<Texture>>,
}

#[derive(BinRead, Copy, Clone, Debug)]
#[br(repr = u32)]
pub enum TextureFormat {
    R5G5B5A1 = 0,
    R4G4B4A4 = 1,
    R5G6B5 = 2,
    R8G8B8A8 = 3,
}

#[derive(BinRead, Debug)]
pub struct Texture {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub unk_c: i32,
    pub unk_10: i32,
    pub unk_14: i32,
    pub unk_18: i32,
    pub texture_format: TextureFormat,
    #[br(args { width, height, texture_format } )]
    pub data: FilePtr32<TextureData>,
}

#[derive(BinrwNamedArgs, Clone, Debug)]
pub struct TextureDataArgs {
    width: u32,
    height: u32,
    texture_format: TextureFormat,
}

pub struct TextureData(pub DynamicImage);

impl Debug for TextureData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextureData")
            .finish()
    }
}

impl BinRead for TextureData {
    type Args = TextureDataArgs;

    fn read_options<R: Read + Seek>(reader: &mut R, options: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        let format = args.texture_format;

        let bpp = match format {
            TextureFormat::R5G5B5A1 => 2,
            TextureFormat::R4G4B4A4 => 2,
            TextureFormat::R5G6B5 => 2,
            TextureFormat::R8G8B8A8 => 4,
        };
        let size = (args.width * args.height * bpp) as usize;
        let mut data = vec![0; size];
        reader.read_exact(&mut data)?;

        let image: DynamicImage = match format {
            TextureFormat::R5G5B5A1 |TextureFormat::R4G4B4A4 |TextureFormat::R5G6B5 => {
                let shorts: &[u16] = bytemuck::cast_slice(&data);

                match format {
                    TextureFormat::R5G5B5A1 | TextureFormat::R4G4B4A4 => {
                        let mut pixels = Vec::new();
                        for &short in shorts.iter() {
                            match format {
                                TextureFormat::R5G5B5A1 => {
                                    pixels.push((((short >> 15) & 0x1F) * 0xFF / 0x1F) as u8);
                                    pixels.push((((short >> 10) & 0x1F) * 0xFF / 0x1F) as u8);
                                    pixels.push((((short >> 5) & 0x1F) * 0xFF / 0x1F) as u8);
                                    pixels.push((((short >> 0) & 0x1F) * 0xFF / 0x1F) as u8);
                                }
                                TextureFormat::R4G4B4A4 => {
                                    pixels.push((((short >> 12) & 0xF) * 0xFF / 0xF) as u8);
                                    pixels.push((((short >> 8) & 0xF) * 0xFF / 0xF) as u8);
                                    pixels.push((((short >> 4) & 0xF) * 0xFF / 0xF) as u8);
                                    pixels.push((((short >> 0) & 0xF) * 0xFF / 0xF) as u8);
                                }
                                _ => unreachable!(),
                            }
                        }
                        RgbaImage::from_vec(args.width, args.height, pixels).unwrap().into()
                    }
                    TextureFormat::R5G6B5 => {
                        let mut pixels = Vec::new();
                        for &short in shorts.iter() {
                            pixels.push((((short >> 11) & 0x1F) * 0xFF / 0x1F) as u8);
                            pixels.push((((short >> 5) & 0x3F) * 0xFF / 0x3F) as u8);
                            pixels.push((((short >> 0) & 0x1F) * 0xFF / 0x1F) as u8);
                        }
                        RgbImage::from_vec(args.width, args.height, pixels).unwrap().into()
                    }
                    _ => unreachable!(),
                }
            }
            TextureFormat::R8G8B8A8 => {
                RgbaImage::from_vec(args.width, args.height, data).unwrap().into()
            }
        };

        // the textures seem to be stored upside-down because OpenGL
        let image = image.flipv();

        Ok(TextureData(image))
    }
}

pub fn read_texture_package(data: &[u8]) -> Result<TexturePackage> {
    Ok(binrw::io::Cursor::new(data).read_le()?)
}