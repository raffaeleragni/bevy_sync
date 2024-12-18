use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use lz4_compression::{compress, decompress};
use serde::{Deserialize, Serialize};
use wgpu_types::{AstcBlock, AstcChannel};

pub(crate) fn bin_to_image(bin: &[u8]) -> Option<Image> {
    let bin = decompress::decompress(bin).unwrap();
    let img = bincode::deserialize::<ImageData>(&bin).ok()?;
    let dimension = match img.dimensions {
        1 => TextureDimension::D1,
        2 => TextureDimension::D2,
        3 => TextureDimension::D3,
        _ => TextureDimension::D2,
    };
    Some(Image::new(
        Extent3d {
            width: img.width,
            height: img.height,
            depth_or_array_layers: img.depth_or_array_layers,
        },
        dimension,
        img.data,
        img.format.0,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}

pub(crate) fn image_to_bin(image: &Image) -> Option<Vec<u8>> {
    let dimensions = match image.texture_descriptor.dimension {
        TextureDimension::D1 => 1,
        TextureDimension::D2 => 2,
        TextureDimension::D3 => 3,
    };
    let img = ImageData {
        width: image.texture_descriptor.size.width,
        height: image.texture_descriptor.size.height,
        depth_or_array_layers: image.texture_descriptor.size.depth_or_array_layers,
        dimensions,
        format: TextureFormatWrapper(image.texture_descriptor.format),
        data: image.data.clone(),
    };
    Some(compress::compress(&bincode::serialize(&img).ok()?))
}

#[derive(Serialize, Deserialize)]
struct ImageData {
    width: u32,
    height: u32,
    depth_or_array_layers: u32,
    dimensions: u8,
    format: TextureFormatWrapper,
    data: Vec<u8>,
}

struct TextureFormatWrapper(TextureFormat);

impl<'de> Deserialize<'de> for TextureFormatWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Error, Unexpected};

        struct TextureFormatVisitor;

        impl de::Visitor<'_> for TextureFormatVisitor {
            type Value = TextureFormat;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid texture format")
            }

            fn visit_str<E: Error>(self, s: &str) -> Result<Self::Value, E> {
                let format = match s {
                    "r8unorm" => TextureFormat::R8Unorm,
                    "r8snorm" => TextureFormat::R8Snorm,
                    "r8uint" => TextureFormat::R8Uint,
                    "r8sint" => TextureFormat::R8Sint,
                    "r16uint" => TextureFormat::R16Uint,
                    "r16sint" => TextureFormat::R16Sint,
                    "r16unorm" => TextureFormat::R16Unorm,
                    "r16snorm" => TextureFormat::R16Snorm,
                    "r16float" => TextureFormat::R16Float,
                    "rg8unorm" => TextureFormat::Rg8Unorm,
                    "rg8snorm" => TextureFormat::Rg8Snorm,
                    "rg8uint" => TextureFormat::Rg8Uint,
                    "rg8sint" => TextureFormat::Rg8Sint,
                    "r32uint" => TextureFormat::R32Uint,
                    "r32sint" => TextureFormat::R32Sint,
                    "r32float" => TextureFormat::R32Float,
                    "rg16uint" => TextureFormat::Rg16Uint,
                    "rg16sint" => TextureFormat::Rg16Sint,
                    "rg16unorm" => TextureFormat::Rg16Unorm,
                    "rg16snorm" => TextureFormat::Rg16Snorm,
                    "rg16float" => TextureFormat::Rg16Float,
                    "rgba8unorm" => TextureFormat::Rgba8Unorm,
                    "rgba8unorm-srgb" => TextureFormat::Rgba8UnormSrgb,
                    "rgba8snorm" => TextureFormat::Rgba8Snorm,
                    "rgba8uint" => TextureFormat::Rgba8Uint,
                    "rgba8sint" => TextureFormat::Rgba8Sint,
                    "bgra8unorm" => TextureFormat::Bgra8Unorm,
                    "bgra8unorm-srgb" => TextureFormat::Bgra8UnormSrgb,
                    "rgb10a2uint" => TextureFormat::Rgb10a2Uint,
                    "rgb10a2unorm" => TextureFormat::Rgb10a2Unorm,
                    "rg11b10ufloat" => TextureFormat::Rg11b10Float,
                    "rg32uint" => TextureFormat::Rg32Uint,
                    "rg32sint" => TextureFormat::Rg32Sint,
                    "rg32float" => TextureFormat::Rg32Float,
                    "rgba16uint" => TextureFormat::Rgba16Uint,
                    "rgba16sint" => TextureFormat::Rgba16Sint,
                    "rgba16unorm" => TextureFormat::Rgba16Unorm,
                    "rgba16snorm" => TextureFormat::Rgba16Snorm,
                    "rgba16float" => TextureFormat::Rgba16Float,
                    "rgba32uint" => TextureFormat::Rgba32Uint,
                    "rgba32sint" => TextureFormat::Rgba32Sint,
                    "rgba32float" => TextureFormat::Rgba32Float,
                    "stencil8" => TextureFormat::Stencil8,
                    "depth32float" => TextureFormat::Depth32Float,
                    "depth32float-stencil8" => TextureFormat::Depth32FloatStencil8,
                    "depth16unorm" => TextureFormat::Depth16Unorm,
                    "depth24plus" => TextureFormat::Depth24Plus,
                    "depth24plus-stencil8" => TextureFormat::Depth24PlusStencil8,
                    "nv12" => TextureFormat::NV12,
                    "rgb9e5ufloat" => TextureFormat::Rgb9e5Ufloat,
                    "bc1-rgba-unorm" => TextureFormat::Bc1RgbaUnorm,
                    "bc1-rgba-unorm-srgb" => TextureFormat::Bc1RgbaUnormSrgb,
                    "bc2-rgba-unorm" => TextureFormat::Bc2RgbaUnorm,
                    "bc2-rgba-unorm-srgb" => TextureFormat::Bc2RgbaUnormSrgb,
                    "bc3-rgba-unorm" => TextureFormat::Bc3RgbaUnorm,
                    "bc3-rgba-unorm-srgb" => TextureFormat::Bc3RgbaUnormSrgb,
                    "bc4-r-unorm" => TextureFormat::Bc4RUnorm,
                    "bc4-r-snorm" => TextureFormat::Bc4RSnorm,
                    "bc5-rg-unorm" => TextureFormat::Bc5RgUnorm,
                    "bc5-rg-snorm" => TextureFormat::Bc5RgSnorm,
                    "bc6h-rgb-ufloat" => TextureFormat::Bc6hRgbUfloat,
                    "bc6h-rgb-float" => TextureFormat::Bc6hRgbFloat,
                    "bc7-rgba-unorm" => TextureFormat::Bc7RgbaUnorm,
                    "bc7-rgba-unorm-srgb" => TextureFormat::Bc7RgbaUnormSrgb,
                    "etc2-rgb8unorm" => TextureFormat::Etc2Rgb8Unorm,
                    "etc2-rgb8unorm-srgb" => TextureFormat::Etc2Rgb8UnormSrgb,
                    "etc2-rgb8a1unorm" => TextureFormat::Etc2Rgb8A1Unorm,
                    "etc2-rgb8a1unorm-srgb" => TextureFormat::Etc2Rgb8A1UnormSrgb,
                    "etc2-rgba8unorm" => TextureFormat::Etc2Rgba8Unorm,
                    "etc2-rgba8unorm-srgb" => TextureFormat::Etc2Rgba8UnormSrgb,
                    "eac-r11unorm" => TextureFormat::EacR11Unorm,
                    "eac-r11snorm" => TextureFormat::EacR11Snorm,
                    "eac-rg11unorm" => TextureFormat::EacRg11Unorm,
                    "eac-rg11snorm" => TextureFormat::EacRg11Snorm,
                    other => {
                        if let Some(parts) = other.strip_prefix("astc-") {
                            let (block, channel) = parts
                                .split_once('-')
                                .ok_or_else(|| E::invalid_value(Unexpected::Str(s), &self))?;

                            let block = match block {
                                "4x4" => AstcBlock::B4x4,
                                "5x4" => AstcBlock::B5x4,
                                "5x5" => AstcBlock::B5x5,
                                "6x5" => AstcBlock::B6x5,
                                "6x6" => AstcBlock::B6x6,
                                "8x5" => AstcBlock::B8x5,
                                "8x6" => AstcBlock::B8x6,
                                "8x8" => AstcBlock::B8x8,
                                "10x5" => AstcBlock::B10x5,
                                "10x6" => AstcBlock::B10x6,
                                "10x8" => AstcBlock::B10x8,
                                "10x10" => AstcBlock::B10x10,
                                "12x10" => AstcBlock::B12x10,
                                "12x12" => AstcBlock::B12x12,
                                _ => return Err(E::invalid_value(Unexpected::Str(s), &self)),
                            };

                            let channel = match channel {
                                "unorm" => AstcChannel::Unorm,
                                "unorm-srgb" => AstcChannel::UnormSrgb,
                                "hdr" => AstcChannel::Hdr,
                                _ => return Err(E::invalid_value(Unexpected::Str(s), &self)),
                            };

                            TextureFormat::Astc { block, channel }
                        } else {
                            return Err(E::invalid_value(Unexpected::Str(s), &self));
                        }
                    }
                };

                Ok(format)
            }
        }

        deserializer.deserialize_str(TextureFormatVisitor).map(TextureFormatWrapper)
    }
}

impl Serialize for TextureFormatWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s: String;
        let name = match self.0 {
            TextureFormat::R8Unorm => "r8unorm",
            TextureFormat::R8Snorm => "r8snorm",
            TextureFormat::R8Uint => "r8uint",
            TextureFormat::R8Sint => "r8sint",
            TextureFormat::R16Uint => "r16uint",
            TextureFormat::R16Sint => "r16sint",
            TextureFormat::R16Unorm => "r16unorm",
            TextureFormat::R16Snorm => "r16snorm",
            TextureFormat::R16Float => "r16float",
            TextureFormat::Rg8Unorm => "rg8unorm",
            TextureFormat::Rg8Snorm => "rg8snorm",
            TextureFormat::Rg8Uint => "rg8uint",
            TextureFormat::Rg8Sint => "rg8sint",
            TextureFormat::R32Uint => "r32uint",
            TextureFormat::R32Sint => "r32sint",
            TextureFormat::R32Float => "r32float",
            TextureFormat::Rg16Uint => "rg16uint",
            TextureFormat::Rg16Sint => "rg16sint",
            TextureFormat::Rg16Unorm => "rg16unorm",
            TextureFormat::Rg16Snorm => "rg16snorm",
            TextureFormat::Rg16Float => "rg16float",
            TextureFormat::Rgba8Unorm => "rgba8unorm",
            TextureFormat::Rgba8UnormSrgb => "rgba8unorm-srgb",
            TextureFormat::Rgba8Snorm => "rgba8snorm",
            TextureFormat::Rgba8Uint => "rgba8uint",
            TextureFormat::Rgba8Sint => "rgba8sint",
            TextureFormat::Bgra8Unorm => "bgra8unorm",
            TextureFormat::Bgra8UnormSrgb => "bgra8unorm-srgb",
            TextureFormat::Rgb10a2Uint => "rgb10a2uint",
            TextureFormat::Rgb10a2Unorm => "rgb10a2unorm",
            TextureFormat::Rg11b10Float => "rg11b10ufloat",
            TextureFormat::Rg32Uint => "rg32uint",
            TextureFormat::Rg32Sint => "rg32sint",
            TextureFormat::Rg32Float => "rg32float",
            TextureFormat::Rgba16Uint => "rgba16uint",
            TextureFormat::Rgba16Sint => "rgba16sint",
            TextureFormat::Rgba16Unorm => "rgba16unorm",
            TextureFormat::Rgba16Snorm => "rgba16snorm",
            TextureFormat::Rgba16Float => "rgba16float",
            TextureFormat::Rgba32Uint => "rgba32uint",
            TextureFormat::Rgba32Sint => "rgba32sint",
            TextureFormat::Rgba32Float => "rgba32float",
            TextureFormat::Stencil8 => "stencil8",
            TextureFormat::Depth32Float => "depth32float",
            TextureFormat::Depth16Unorm => "depth16unorm",
            TextureFormat::Depth32FloatStencil8 => "depth32float-stencil8",
            TextureFormat::Depth24Plus => "depth24plus",
            TextureFormat::Depth24PlusStencil8 => "depth24plus-stencil8",
            TextureFormat::NV12 => "nv12",
            TextureFormat::Rgb9e5Ufloat => "rgb9e5ufloat",
            TextureFormat::Bc1RgbaUnorm => "bc1-rgba-unorm",
            TextureFormat::Bc1RgbaUnormSrgb => "bc1-rgba-unorm-srgb",
            TextureFormat::Bc2RgbaUnorm => "bc2-rgba-unorm",
            TextureFormat::Bc2RgbaUnormSrgb => "bc2-rgba-unorm-srgb",
            TextureFormat::Bc3RgbaUnorm => "bc3-rgba-unorm",
            TextureFormat::Bc3RgbaUnormSrgb => "bc3-rgba-unorm-srgb",
            TextureFormat::Bc4RUnorm => "bc4-r-unorm",
            TextureFormat::Bc4RSnorm => "bc4-r-snorm",
            TextureFormat::Bc5RgUnorm => "bc5-rg-unorm",
            TextureFormat::Bc5RgSnorm => "bc5-rg-snorm",
            TextureFormat::Bc6hRgbUfloat => "bc6h-rgb-ufloat",
            TextureFormat::Bc6hRgbFloat => "bc6h-rgb-float",
            TextureFormat::Bc7RgbaUnorm => "bc7-rgba-unorm",
            TextureFormat::Bc7RgbaUnormSrgb => "bc7-rgba-unorm-srgb",
            TextureFormat::Etc2Rgb8Unorm => "etc2-rgb8unorm",
            TextureFormat::Etc2Rgb8UnormSrgb => "etc2-rgb8unorm-srgb",
            TextureFormat::Etc2Rgb8A1Unorm => "etc2-rgb8a1unorm",
            TextureFormat::Etc2Rgb8A1UnormSrgb => "etc2-rgb8a1unorm-srgb",
            TextureFormat::Etc2Rgba8Unorm => "etc2-rgba8unorm",
            TextureFormat::Etc2Rgba8UnormSrgb => "etc2-rgba8unorm-srgb",
            TextureFormat::EacR11Unorm => "eac-r11unorm",
            TextureFormat::EacR11Snorm => "eac-r11snorm",
            TextureFormat::EacRg11Unorm => "eac-rg11unorm",
            TextureFormat::EacRg11Snorm => "eac-rg11snorm",
            TextureFormat::Astc { block, channel } => {
                let block = match block {
                    AstcBlock::B4x4 => "4x4",
                    AstcBlock::B5x4 => "5x4",
                    AstcBlock::B5x5 => "5x5",
                    AstcBlock::B6x5 => "6x5",
                    AstcBlock::B6x6 => "6x6",
                    AstcBlock::B8x5 => "8x5",
                    AstcBlock::B8x6 => "8x6",
                    AstcBlock::B8x8 => "8x8",
                    AstcBlock::B10x5 => "10x5",
                    AstcBlock::B10x6 => "10x6",
                    AstcBlock::B10x8 => "10x8",
                    AstcBlock::B10x10 => "10x10",
                    AstcBlock::B12x10 => "12x10",
                    AstcBlock::B12x12 => "12x12",
                };

                let channel = match channel {
                    AstcChannel::Unorm => "unorm",
                    AstcChannel::UnormSrgb => "unorm-srgb",
                    AstcChannel::Hdr => "hdr",
                };

                s = format!("astc-{block}-{channel}");
                &s
            }
        };
        serializer.serialize_str(name)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_image() {
        let img = Image::default();
        let bin = image_to_bin(&img).unwrap();
        let img2 = bin_to_image(&bin).unwrap();
        assert_eq!(img.data, img2.data);
    }
}
