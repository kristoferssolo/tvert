use crate::{Error, PixelSize};
use std::ops::Range;

/// A validated nonempty region of a tightly packed RGBA image
pub struct RgbaRegion {
    row_bytes: usize,
    columns: Range<usize>,
    rows: Range<usize>,
    size: PixelSize,
}

impl RgbaRegion {
    pub fn new(size: PixelSize, x: Range<usize>, y: Range<usize>) -> Result<Option<Self>, Error> {
        let (width, height) = size.try_into()?;
        if x.is_empty() || y.is_empty() || x.end > width || y.end > height {
            return Ok(None);
        }
        let start = x.start.checked_mul(4).ok_or(Error::SizeOverflow)?;
        let end = x.end.checked_mul(4).ok_or(Error::SizeOverflow)?;
        let width = u32::try_from(x.len()).map_err(|_| Error::SizeOverflow)?;
        let height = u32::try_from(y.len()).map_err(|_| Error::SizeOverflow)?;
        Ok(Some(Self {
            row_bytes: size.row_byes()?,
            columns: start..end,
            rows: y,
            size: PixelSize::try_from((width, height))?,
        }))
    }

    pub const fn size(&self) -> PixelSize {
        self.size
    }

    pub fn rows<'a>(&self, rgba: &'a [u8]) -> impl Iterator<Item = Result<&'a [u8], Error>> {
        let columns = self.columns.clone();
        rgba.chunks_exact(self.row_bytes)
            .skip(self.rows.start)
            .take(self.rows.len())
            .map(move |row| row.get(columns.clone()).ok_or(Error::SizeOverflow))
    }

    pub fn rows_mut<'a>(
        &self,
        rgba: &'a mut [u8],
    ) -> impl Iterator<Item = Result<&'a mut [u8], Error>> {
        let columns = self.columns.clone();
        rgba.chunks_exact_mut(self.row_bytes)
            .skip(self.rows.start)
            .take(self.rows.len())
            .map(move |row| row.get_mut(columns.clone()).ok_or(Error::SizeOverflow))
    }
}
