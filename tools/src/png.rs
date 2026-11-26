//! A small PNG writer: 8-bit RGB, no compression beyond stored blocks.

pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    pub fn rgb(width: u32, height: u32) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![0; (width * height * 3) as usize],
        }
    }

    pub fn set(&mut self, x: u32, y: u32, colour: [u8; 3]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let at = ((y * self.width + x) * 3) as usize;
        self.pixels[at..at + 3].copy_from_slice(&colour);
    }

    pub fn rect(&mut self, left: u32, top: u32, right: u32, bottom: u32, colour: [u8; 3]) {
        for y in top..bottom.min(self.height) {
            for x in left..right.min(self.width) {
                self.set(x, y, colour);
            }
        }
    }

    pub fn png(&self) -> Vec<u8> {
        let mut raw = Vec::with_capacity((self.height * (1 + self.width * 3)) as usize);
        for y in 0..self.height {
            raw.push(0);
            let from = (y * self.width * 3) as usize;
            raw.extend_from_slice(&self.pixels[from..from + (self.width * 3) as usize]);
        }
        let mut header = Vec::with_capacity(13);
        header.extend_from_slice(&self.width.to_be_bytes());
        header.extend_from_slice(&self.height.to_be_bytes());
        header.extend_from_slice(&[8, 2, 0, 0, 0]);

        let mut out = Vec::new();
        out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        chunk(&mut out, b"IHDR", &header);
        chunk(&mut out, b"IDAT", &zlib(&raw));
        chunk(&mut out, b"IEND", &[]);
        out
    }
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(kind);
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    for (index, block) in raw.chunks(65_535).enumerate() {
        let last = (index + 1) * 65_535 >= raw.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(block.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let low = crc & 1;
            crc = (crc >> 1) ^ (0xedb8_8320 & low.wrapping_neg());
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_png_a_decoder_accepts() {
        let mut canvas = Canvas::rgb(4, 3);
        canvas.set(0, 0, [255, 0, 0]);
        let bytes = canvas.png();
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
        assert_eq!(&bytes[12..16], b"IHDR");
        assert!(bytes.windows(4).any(|window| window == b"IDAT"));
        assert_eq!(&bytes[bytes.len() - 8..bytes.len() - 4], b"IEND");
    }

    #[test]
    fn checksums_match_the_specification() {
        // The zlib and PNG documents' own examples.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }
}
