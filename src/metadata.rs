use std::{fs::File, io::BufReader, path::Path};

use flate2::read::ZlibDecoder;
use std::io::Read;

#[derive(Debug)]
pub struct ImageMetadata {
    pub exif: Vec<ExifField>,
    pub xmp: Option<String>,
}

#[derive(Debug)]
pub struct ExifField {
    pub group: String,
    pub name: String,
    pub value: String,
}

impl ImageMetadata {
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes =
            std::fs::read(path).map_err(|error| format!("画像を読み込めませんでした: {error}"))?;
        let exif = read_exif(path)?;
        let xmp = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            xmp_from_png(&bytes)
        } else if bytes.starts_with(&[0xff, 0xd8]) {
            xmp_from_jpeg(&bytes)
        } else {
            None
        };
        Ok(Self { exif, xmp })
    }
}

fn read_exif(path: &Path) -> Result<Vec<ExifField>, String> {
    let file = File::open(path).map_err(|error| format!("画像を開けませんでした: {error}"))?;
    let mut reader = BufReader::new(file);
    let exif = match exif::Reader::new().read_from_container(&mut reader) {
        Ok(exif) => exif,
        Err(exif::Error::NotFound(_)) => return Ok(Vec::new()),
        Err(error) => return Err(format!("Exifを解析できませんでした: {error}")),
    };

    Ok(exif
        .fields()
        .map(|field| ExifField {
            group: format!("{:?}", field.ifd_num),
            name: field.tag.to_string(),
            value: field.display_value().with_unit(&exif).to_string(),
        })
        .collect())
}

fn xmp_from_jpeg(bytes: &[u8]) -> Option<String> {
    const XMP_HEADER: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
    let mut offset = 2;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            break;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        let length = usize::from(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]));
        if length < 2 || offset + length > bytes.len() {
            break;
        }
        let data = &bytes[offset + 2..offset + length];
        if marker == 0xe1 && data.starts_with(XMP_HEADER) {
            return decode_xmp(&data[XMP_HEADER.len()..]);
        }
        offset += length;
    }
    None
}

fn xmp_from_png(bytes: &[u8]) -> Option<String> {
    let mut offset = 8;
    while offset + 12 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start.checked_add(length)?;
        if data_end + 4 > bytes.len() {
            break;
        }
        let data = &bytes[data_start..data_end];
        let result = match kind {
            b"iTXt" => xmp_from_itxt(data),
            b"tEXt" => xmp_from_text(data),
            b"zTXt" => xmp_from_ztxt(data),
            _ => None,
        };
        if result.is_some() {
            return result;
        }
        offset = data_end + 4;
    }
    None
}

fn xmp_from_itxt(data: &[u8]) -> Option<String> {
    let keyword_end = data.iter().position(|byte| *byte == 0)?;
    if &data[..keyword_end] != b"XML:com.adobe.xmp" {
        return None;
    }
    let mut cursor = keyword_end + 1;
    let compressed = *data.get(cursor)? == 1;
    cursor += 2; // compression flag + compression method
    for _ in 0..2 {
        cursor += data.get(cursor..)?.iter().position(|byte| *byte == 0)? + 1;
    }
    let payload = data.get(cursor..)?;
    if compressed {
        inflate_xmp(payload)
    } else {
        decode_xmp(payload)
    }
}

fn xmp_from_text(data: &[u8]) -> Option<String> {
    let separator = data.iter().position(|byte| *byte == 0)?;
    (&data[..separator] == b"XML:com.adobe.xmp")
        .then(|| decode_xmp(&data[separator + 1..]))
        .flatten()
}

fn xmp_from_ztxt(data: &[u8]) -> Option<String> {
    let separator = data.iter().position(|byte| *byte == 0)?;
    if &data[..separator] != b"XML:com.adobe.xmp" || *data.get(separator + 1)? != 0 {
        return None;
    }
    inflate_xmp(&data[separator + 2..])
}

fn inflate_xmp(payload: &[u8]) -> Option<String> {
    let mut decoder = ZlibDecoder::new(payload);
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes).ok()?;
    decode_xmp(&bytes)
}

fn decode_xmp(bytes: &[u8]) -> Option<String> {
    let value = String::from_utf8_lossy(bytes)
        .trim_matches(|character: char| character == '\0' || character.is_whitespace())
        .to_owned();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_jpeg_xmp() {
        let payload = b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta>test</x:xmpmeta>";
        let length = (payload.len() + 2) as u16;
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
        jpeg.extend_from_slice(&length.to_be_bytes());
        jpeg.extend_from_slice(payload);
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        assert_eq!(
            xmp_from_jpeg(&jpeg).as_deref(),
            Some("<x:xmpmeta>test</x:xmpmeta>")
        );
    }

    #[test]
    fn extracts_png_itxt_xmp() {
        let mut data = b"XML:com.adobe.xmp\0\0\0\0\0".to_vec();
        data.extend_from_slice("<x:xmpmeta>撮影場所：東京</x:xmpmeta>".as_bytes());
        assert_eq!(
            xmp_from_itxt(&data).as_deref(),
            Some("<x:xmpmeta>撮影場所：東京</x:xmpmeta>")
        );
    }
}
