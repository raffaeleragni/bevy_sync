use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use lz4_compress::{compress, decompress};
use serde::{Deserialize, Serialize};

pub(crate) fn bin_to_image(bin: &[u8]) -> Option<Image> {
    let bin = decompress(bin).unwrap();
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
        img.format,
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
        format: image.texture_descriptor.format,
        data: image.data.clone(),
    };
    Some(compress(&bincode::serialize(&img).ok()?))
}

#[derive(Serialize, Deserialize)]
struct ImageData {
    width: u32,
    height: u32,
    depth_or_array_layers: u32,
    dimensions: u8,
    format: TextureFormat,
    data: Vec<u8>,
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
