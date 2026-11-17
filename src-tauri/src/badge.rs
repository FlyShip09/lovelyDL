//! The taskbar badge: a red disc with the number of downloads that are not finished, drawn
//! as a 32 × 32 overlay icon (Windows scales it for the taskbar).

const SIZE: usize = 32;

/// 5 × 7 bitmaps of the digits, one row per byte, most significant of the low 5 bits on the left.
const DIGITS: [[u8; 7]; 10] = [
    [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
    [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
    [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
    [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
    [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
    [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
    [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
    [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
    [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
    [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
];

/// RGBA pixels of the badge for `count` (shown up to 99).
pub fn render(count: u32) -> Vec<u8> {
    let mut px = vec![0u8; SIZE * SIZE * 4];
    // The disc, with a one-pixel soft edge so it is not jagged.
    let center = SIZE as f32 / 2.0;
    let radius = center - 0.5;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = ((x as f32 + 0.5 - center).powi(2) + (y as f32 + 0.5 - center).powi(2)).sqrt();
            let coverage = (radius - d + 0.5).clamp(0.0, 1.0);
            let i = (y * SIZE + x) * 4;
            px[i..i + 4].copy_from_slice(&[0xE8, 0x1E, 0x2C, (coverage * 255.0) as u8]);
        }
    }
    let text = count.min(99).to_string();
    let digits: Vec<usize> = text.bytes().map(|b| usize::from(b - b'0')).collect();
    let scale = if digits.len() == 1 { 3 } else { 2 };
    let gap = scale;
    let width = digits.len() * 5 * scale + (digits.len() - 1) * gap;
    let (left, top) = ((SIZE - width) / 2, (SIZE - 7 * scale) / 2);
    for (n, digit) in digits.iter().enumerate() {
        let x0 = left + n * (5 * scale + gap);
        for (row, bits) in DIGITS[*digit].iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let (x, y) = (x0 + col * scale + dx, top + row * scale + dy);
                        let i = (y * SIZE + x) * 4;
                        px[i..i + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
                    }
                }
            }
        }
    }
    px
}

pub fn size() -> u32 {
    SIZE as u32
}
