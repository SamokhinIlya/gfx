use crate::math::Num;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}
static_assertions::assert_eq_size!(Color, u32);

pub fn set_intensity(c: Color, i: Num) -> Color {
    fn clamp(x: Num, range: std::ops::RangeInclusive<Num>) -> Num {
        let start = *range.start();
        let end = *range.end();
        if x < start {
            start
        } else if x > end {
            end
        } else {
            x
        }
    }

    // i is guaranteed to be in range 0.0..=1.0
    fn set_channel_intensity(c: u8, i: Num) -> u8 {
        (c as Num * i) as u8
    }

    let i = clamp(i, 0.0..=1.0);

    Color {
        b: set_channel_intensity(c.b, i),
        g: set_channel_intensity(c.g, i),
        r: set_channel_intensity(c.r, i),
        ..c
    }
}

pub struct Canvas {
    width: usize,
    height: usize,
    data: *mut Color,
}

impl Drop for Canvas {
    fn drop(&mut self) {
        use std::alloc::{Layout, dealloc};

        unsafe {
            dealloc(
                self.data as *mut _,
                Layout::from_size_align_unchecked(
                    self.width * self.height * std::mem::size_of_val(&*self.data),
                    std::mem::align_of_val(&*self.data)
                )
            )
        }
    }
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Result<Self, std::alloc::LayoutErr> {
        use std::alloc::{Layout, alloc_zeroed};

        Ok(Self {
            width,
            height,
            data: unsafe { alloc_zeroed(Layout::array::<Color>(width * height)?) } as *mut _,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub unsafe fn data(&self) -> *mut Color {
        self.data
    }

    pub fn set(&mut self, (x, y): (usize, usize), pxl: Color) {
        debug_assert!(x < self.width, "Canvas::set. x: {} >= self.width: {}", x, self.width);
        debug_assert!(y < self.height, "Canvas::set. y: {} >= self.height: {}", y, self.height);

        unsafe {
            *self.data.add(x + self.width * y) = pxl;
        }
    }
}