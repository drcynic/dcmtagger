use std::io::{self, Write};

use base64::{Engine, engine::general_purpose::STANDARD};
use crossterm::{cursor::MoveTo, execute};
use ratatui::layout::Rect;

const IMAGE_ID: u32 = 100;
const PLACEMENT_ID: u32 = 100;

#[derive(Debug, Default)]
pub struct KittyImage {
    data: String,
    width: u32,
    height: u32,
}

impl KittyImage {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer = vec![128u8; (width * height * 3) as usize];

        Self {
            data: STANDARD.encode(&buffer),
            width,
            height,
        }
    }

    pub fn upload(&self, out: &mut impl Write) -> io::Result<()> {
        write!(
            out,
            "\x1b_Ga=t,f=24,s={},v={},i={},q=2,m=0;{}\x1b\\",
            self.width, self.height, IMAGE_ID, self.data
        )?;

        out.flush()
    }

    pub fn place(&self, out: &mut impl Write, area: Rect) -> io::Result<()> {
        if area.width == 0 || area.height == 0 {
            return Ok(());
        }

        let image_ratio = self.width as f64 / self.height as f64;

        // Account for the fact that terminal cells are taller
        // than they are wide.
        //
        // WezTerm's actual cell ratio is preferable to a hardcoded
        // value; this is a reasonable default.
        const CELL_WIDTH_OVER_HEIGHT: f64 = 0.5;

        let available_ratio = area.width as f64 * CELL_WIDTH_OVER_HEIGHT / area.height as f64;

        let (x, y, columns, rows) = if image_ratio > available_ratio {
            // Width is the limiting dimension.
            let columns = area.width;
            let rows = (columns as f64 * CELL_WIDTH_OVER_HEIGHT / image_ratio).round().max(1.0) as u16;
            let y = area.y + area.height.saturating_sub(rows) / 2;

            (area.x, y, columns, rows)
        } else {
            // Height is the limiting dimension.
            let rows = area.height;
            let columns = (rows as f64 * image_ratio / CELL_WIDTH_OVER_HEIGHT).round().max(1.0) as u16;
            let x = area.x + area.width.saturating_sub(columns) / 2;

            (x, area.y, columns, rows)
        };

        execute!(out, MoveTo(x, y))?;

        write!(
            out,
            "\x1b_Ga=p,i={},p={},c={},r={},C=1,q=2\x1b\\",
            IMAGE_ID, PLACEMENT_ID, columns, rows,
        )?;

        out.flush()
    }

    pub fn hide(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "\x1b_Ga=d,d=i,i={},p={}\x1b\\", IMAGE_ID, PLACEMENT_ID,)?;
        out.flush()
    }

    pub fn delete(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "\x1b_Ga=d,d=I,i={}\x1b\\", IMAGE_ID)?;

        out.flush()
    }
}
