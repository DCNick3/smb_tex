use std::fmt::Debug;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use anyhow::{bail, Result};
use binrw::{BinRead, binread, BinReaderExt, BinResult, BinrwNamedArgs, BinWrite, FilePtr32, ReadOptions};
use image::{DynamicImage, RgbaImage, RgbImage};
use serde::{Deserialize, Serialize};

#[binread]
#[derive(Debug)]
pub struct TexturePackage {
    #[br(temp)]
    texture_count: u32,
    #[br(parse_with = FilePtr32::parse, count = texture_count)]
    pub textures: Vec<Texture>,
}

impl TexturePackage {
    pub fn from_directory(path: &Path) -> Result<Self> {
        let mut textures = Vec::new();

        let items = std::fs::read_dir(path)?;
        for item in items {
            let item = item?;
            let path = item.path();
            if path.is_dir() {
                continue;
            }
            if path.extension().and_then(|v| v.to_str()) != Some("png") {
                continue;
            }
            let meta_path = path.with_extension("json");
            if !meta_path.exists() {
                bail!("Missing meta file for texture {}", path.display());
            }
            let meta: TextureMeta = serde_json::from_slice(&std::fs::read(meta_path)?)?;
            let data = image::open(path)?.into_rgba8();
            let texture = Texture {
                meta,
                data,
            };
            textures.push(texture);
        }

        Ok(Self {
            textures,
        })
    }
}

#[derive(BinWrite, Debug)]
pub struct TexturePackageHeaderRaw {
    pub texture_count: u32,
    pub textures_ptr: u32,
}

#[derive(BinRead, BinWrite, Copy, Clone, Debug, Serialize, Deserialize, clap::ValueEnum)]
#[brw(repr = u32)]
pub enum TextureFormat {
    R5G5B5A1 = 0,
    R4G4B4A4 = 1,
    R5G6B5 = 2,
    R8G8B8A8 = 3,
}

#[derive(BinRead, BinWrite, Debug)]
pub struct TextureHeader {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub unk_c: i32,
    pub unk_10: i32,
    pub unk_14: i32,
    pub unk_18: i32,
    pub texture_format: TextureFormat,
}

impl TextureHeader {
    pub fn meta(&self) -> TextureMeta {
        TextureMeta {
            id: self.id,
            unk_c: self.unk_c,
            unk_10: self.unk_10,
            unk_14: self.unk_14,
            unk_18: self.unk_18,
            texture_format: self.texture_format,
        }
    }

    pub fn data_size(&self) -> u32 {
        self.meta().data_size(self.width, self.height)
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct TextureMeta {
    pub id: u32,
    pub unk_c: i32,
    pub unk_10: i32,
    pub unk_14: i32,
    pub unk_18: i32,
    pub texture_format: TextureFormat,
}

pub fn data_size(format: TextureFormat, width: u32, height: u32) -> u32 {
    let bpp = match format {
        TextureFormat::R5G5B5A1 => 2,
        TextureFormat::R4G4B4A4 => 2,
        TextureFormat::R5G6B5 => 2,
        TextureFormat::R8G8B8A8 => 4,
    };
    width * height * bpp
}

impl TextureMeta {
    pub fn data_size(&self, width: u32, height: u32) -> u32 {
        data_size(self.texture_format, width, height)
    }
}

#[binread]
#[derive(Debug)]
pub struct Texture {
    #[br(temp)]
    pub header: TextureHeader,
    #[br(map = |_: ()| header.meta())]
    pub meta: TextureMeta,
    #[br(parse_with = &FilePtr32::parse_with(read_texture_data), args { width: header.width, height: header.height, texture_format: header.texture_format })]
    pub data: RgbaImage,
}

#[derive(BinrwNamedArgs, Clone, Debug)]
pub struct TextureDataArgs {
    width: u32,
    height: u32,
    texture_format: TextureFormat,
}

fn read_texture_data<R: Read + Seek>(reader: &mut R, _options: &ReadOptions, args: TextureDataArgs) -> BinResult<RgbaImage> {
    let format = args.texture_format;

    let size = data_size(format, args.width, args.height) as usize;
    let mut data = vec![0; size];
    reader.read_exact(&mut data)?;

    let image: RgbaImage = match format {
        TextureFormat::R5G5B5A1 |TextureFormat::R4G4B4A4 |TextureFormat::R5G6B5 => {
            let shorts: &[u16] = bytemuck::cast_slice(&data);

            match format {
                TextureFormat::R5G5B5A1 | TextureFormat::R4G4B4A4 => {
                    let mut pixels = Vec::new();
                    for &short in shorts.iter() {
                        match format {
                            TextureFormat::R5G5B5A1 => {
                                pixels.push( (((short >> 11) & 0x1F) * 0xFF / 0x1F) as u8);
                                pixels.push( (((short >> 6) & 0x1F) * 0xFF / 0x1F) as u8);
                                pixels.push( (((short >> 1) & 0x1F) * 0xFF / 0x1F) as u8);
                                pixels.push( (((short >> 0) & 0x1) * 0xFF) as u8);
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
                    RgbaImage::from_vec(args.width, args.height, pixels).unwrap()
                }
                TextureFormat::R5G6B5 => {
                    let mut pixels = Vec::new();
                    for &short in shorts.iter() {
                        pixels.push((((short >> 11) & 0x1F) * 0xFF / 0x1F) as u8);
                        pixels.push((((short >> 5) & 0x3F) * 0xFF / 0x3F) as u8);
                        pixels.push((((short >> 0) & 0x1F) * 0xFF / 0x1F) as u8);
                    }
                    DynamicImage::from(RgbImage::from_vec(args.width, args.height, pixels).unwrap()).into_rgba8()
                }
                _ => unreachable!(),
            }
        }
        TextureFormat::R8G8B8A8 => {
            RgbaImage::from_vec(args.width, args.height, data).unwrap()
        }
    };

    // the textures seem to be stored upside-down because OpenGL
    let image = DynamicImage::from(image).flipv().into_rgba8();

    Ok(image)
}

pub fn read_texture_package(data: &[u8]) -> Result<TexturePackage> {
    Ok(binrw::io::Cursor::new(data).read_le()?)
}

pub fn write_texture_package(data: &TexturePackage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let header = TexturePackageHeaderRaw {
        texture_count: data.textures.len() as u32,
        textures_ptr: 0x20,
    };
    const TEX_HEADER_SIZE: u32 = 36;
    let mut data_offset = 0x20 + data.textures.len() as u32 * TEX_HEADER_SIZE;

    let mut cur = std::io::Cursor::new(&mut buf);
    header.write_le(&mut cur)?;
    cur.seek(SeekFrom::Start(0x20))?;
    for texture in &data.textures {
        let data_size = texture.meta.data_size(texture.data.width(), texture.data.height());
        let header = TextureHeader {
            id: texture.meta.id,
            width: texture.data.width(),
            height: texture.data.height(),
            texture_format: texture.meta.texture_format,
            unk_c: texture.meta.unk_c,
            unk_10: texture.meta.unk_10,
            unk_14: texture.meta.unk_14,
            unk_18: texture.meta.unk_18,
        };

        header.write_le(&mut cur)?;
        data_offset.write_le(&mut cur)?;

        data_offset += data_size;
    }

    assert_eq!(cur.position(), 0x20 + data.textures.len() as u64 * TEX_HEADER_SIZE as u64);

    for texture in &data.textures {
        let format = texture.meta.texture_format;
        let data = &texture.data;
        for row in data.rows().rev() {
            for pix in row {
                match format {
                    TextureFormat::R5G5B5A1 => {
                        let r = (pix[0] as u16 * 0x1F / 0xFF) << 11;
                        let g = (pix[1] as u16 * 0x1F / 0xFF) << 6;
                        let b = (pix[2] as u16 * 0x1F / 0xFF) << 1;
                        let a = (pix[3] as u16 * 0x1 / 0xFF) << 0;
                        let short = r | g | b | a;
                        short.write_le(&mut cur)?;
                    }
                    TextureFormat::R4G4B4A4 => {
                        let r = (pix[0] as u16 * 0xF / 0xFF) << 12;
                        let g = (pix[1] as u16 * 0xF / 0xFF) << 8;
                        let b = (pix[2] as u16 * 0xF / 0xFF) << 4;
                        let a = (pix[3] as u16 * 0xF / 0xFF) << 0;
                        let short = r | g | b | a;
                        short.write_le(&mut cur)?;
                    }
                    TextureFormat::R8G8B8A8 => {
                        pix[0].write_le(&mut cur)?;
                        pix[1].write_le(&mut cur)?;
                        pix[2].write_le(&mut cur)?;
                        pix[3].write_le(&mut cur)?;
                    }
                    TextureFormat::R5G6B5 => {
                        let r = (pix[0] as u16 * 0x1F / 0xFF) << 11;
                        let g = (pix[1] as u16 * 0x3F / 0xFF) << 5;
                        let b = (pix[2] as u16 * 0x1F / 0xFF) << 0;
                        let short = r | g | b;
                        short.write_le(&mut cur)?;
                    }
                }
            }
        }
    }

    Ok(buf)
}