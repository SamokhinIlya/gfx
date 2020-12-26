use std::mem::swap;
use winapi::{
    ctypes::c_int,
    shared::{
        minwindef::{
            LPARAM,
            LRESULT,
            UINT,
            WPARAM
        },
        windef::{
            HDC,
            HWND,
        },
    },
    um::{
        wingdi::BITMAPINFO,
        winuser::DefWindowProcA,
    },
};
use gfx::win_except::*;

fn main() {
    use std::ffi::CStr;
    use winapi::um::libloaderapi::GetModuleHandleA;
    use winapi::um::winuser::{
        WNDCLASSA,
        AdjustWindowRectEx,
        GetDC,
        CreateWindowExA,
        RegisterClassA,
        WS_CAPTION,
        WS_SYSMENU,
        WS_VISIBLE,
        CW_USEDEFAULT,
    };
    use winapi::um::wingdi::{
        BITMAPINFOHEADER,
        BI_RGB,
    };
    use winapi::shared::minwindef::{MAKELONG};
    use winapi::shared::windef::RECT;

    // gets current .exe module handle. Should pass module name to use in .dll
    let instance_handle = unsafe { GetModuleHandleA(std::ptr::null()) };
    win_except(instance_handle, "GetModuleHandleA(null) failed");

    let window_class_name = unsafe { &CStr::from_bytes_with_nul_unchecked(b"gfx\0") };

    // TODO: use WNDCLASSEX for small icon
    let window_class = WNDCLASSA {
        style: 0,
        lpfnWndProc: Some(window_procedure),

        // TODO: number of extra bytes to allocate following the class struct. What is this for?
        cbClsExtra: 0,

        // TODO: number of extra bytes to allocate following the window instance. What is this for?
        cbWndExtra: 0,
        hInstance: instance_handle,

        // TODO: these are handles to icon/cursor resources. Use a resource or is there an another way?
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),

        // some brush stuff. We draw background ourselves
        hbrBackground: std::ptr::null_mut(),

        // no menu resource
        lpszMenuName: std::ptr::null_mut(),

        lpszClassName: window_class_name.as_ptr(),
    };

    let window_class_atom = unsafe { RegisterClassA(&window_class as *const _) };
    win_except(window_class_atom, "RegisterClassA(...) failed");

    let window_caption = unsafe { &CStr::from_bytes_with_nul_unchecked(b"gfx\0") };

    // TODO: check other styles
    let window_style = WS_CAPTION | WS_SYSMENU | WS_VISIBLE;

    // get window size for desired client area size
    let (width, height) = (1280, 720);
    let (window_width, window_height) = {
        let mut rect = RECT { left: 0, top: 0, right: width, bottom: height };
        win_except(
            unsafe { AdjustWindowRectEx(&mut rect, window_style, 0, 0) },
            "AdjustWindowRectEx(...) failed",
        );
        (rect.right - rect.left, rect.bottom - rect.top)
    };

    let hwnd = unsafe { CreateWindowExA(
        0, // TODO: check extended styles
        MAKELONG(window_class_atom, 0) as *const _,
        window_caption.as_ptr(),
        window_style,
        CW_USEDEFAULT, // x
        CW_USEDEFAULT, // y
        window_width,
        window_height,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        instance_handle,
        std::ptr::null_mut()
    ) };
    win_except(hwnd, "CreateWindowExA(...) failed");

    let device_context = unsafe { GetDC(hwnd) };
    win_except(device_context, "GetDC(hwnd) failed. There is no mention of GetLastError in MSDN");

    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // negative means that bitmap is top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..unsafe { std::mem::zeroed() }
        },
        ..unsafe { std::mem::zeroed() }
    };

    let mut canvas = Canvas::new(width as usize, height as usize).expect("Canvas::new(width, height) failed");

    let font_data = include_bytes!("../data/Inconsolata-Regular.ttf");
    let font = rusttype::Font::try_from_bytes(font_data).expect("font data invalid");
    let scale = rusttype::Scale::uniform(20.0);

    let mut elapsed_history = std::collections::VecDeque::<f64>::with_capacity(500);

    let mut instant = std::time::Instant::now();
    while dispatch_messages() {
        let elapsed = instant.elapsed().as_secs_f64();
        instant = std::time::Instant::now();

        if elapsed_history.len() == elapsed_history.capacity() {
            elapsed_history.pop_front();
        }
        elapsed_history.push_back(elapsed);

        for x in 0..canvas.width {
            for y in 0..canvas.height {
                canvas.set((x, y), Pixel { r: 0, g: 0, b: 0, a: 255 });
            }
        }

        {
            let x = 0;
            let y = 50;
            let width = 300;
            let height = 200;
            draw_frame_time_graph(&mut canvas, (x, y), (width, height), &elapsed_history);
        }

        {
            let elapsed_ms = elapsed * 1000.0;
            let fps = elapsed.recip();
            draw_str(&mut canvas, &format!("{:8.3} ms per frame", elapsed_ms), &font, scale, rusttype::point(0.0, 0.0));
            draw_str(&mut canvas, &format!("{:8.3} fps", fps), &font, scale, rusttype::point(0.0, 20.0));
        }
        stretch_di_bits_win_except(device_context, width, height, &canvas, &bitmap_info);
    }
}

fn draw_frame_time_graph(
    canvas: &mut Canvas,
    (graph_x, graph_y): (usize, usize),
    (width, height): (usize, usize),
    elapsed_history: &std::collections::VecDeque<f64>
) {
    draw_line(canvas, (graph_x as isize, graph_y as isize), ((graph_x + width) as isize, graph_y as isize));
    draw_line(canvas, (graph_x as isize, (graph_y + height) as isize), ((graph_x + width) as isize, (graph_y + height) as isize));
    draw_line(canvas, (graph_x as isize, graph_y as isize), (graph_x as isize, (graph_y + height) as isize));
    draw_line(canvas, ((graph_x + width) as isize, graph_y as isize), ((graph_x + width) as isize, (graph_y + height) as isize));

    let elapsed_ms_history = elapsed_history.iter().map(|x| x * 1000.0);

    let min_elapsed_ms = elapsed_ms_history.clone().fold(f64::MAX, f64::min);
    let max_elapsed_ms = elapsed_ms_history.fold(f64::MIN, f64::max);

    let margin = 10;

    let min_x = (graph_x + margin) as f64;
    let max_x = (graph_x + width - margin) as f64;

    let min_y = (graph_y + margin) as f64;
    let max_y = (graph_y + height - margin) as f64;

    for slice in elapsed_history.iter()
        .rev()
        .enumerate()
        .map(|(i, elapsed)| (
            map(i as f64, 0.0..((elapsed_history.capacity() - 1) as f64), max_x..min_x) as isize,
            map(elapsed * 1000.0, min_elapsed_ms..max_elapsed_ms, max_y..min_y) as isize,
        ))
        .collect::<Vec<_>>()
        .windows(2)
    {
        if let &[(x_prev, y_prev), (x, y)] = slice {
            draw_line(canvas, (x_prev, y_prev), (x, y));
        }
    }

    fn map(x: f64, from: std::ops::Range<f64>, to: std::ops::Range<f64>) -> f64 {
        to.start + (x - from.start) * ((to.end - to.start) / (from.end - from.start))
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Pixel {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
}
static_assertions::assert_eq_size!(Pixel, u32);

struct Canvas {
    width: usize,
    height: usize,
    data: *mut Pixel,
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
    fn new(width: usize, height: usize) -> Result<Self, std::alloc::LayoutErr> {
        use std::alloc::{Layout, alloc_zeroed};

        Ok(Self {
            width,
            height,
            data: unsafe { alloc_zeroed(Layout::array::<Pixel>(width * height)?) } as *mut _,
        })
    }

    fn set(&mut self, (x, y): (usize, usize), pxl: Pixel) {
        debug_assert!(x < self.width, "Canvas::set. x: {} >= self.width: {}", x, self.width);
        debug_assert!(y < self.height, "Canvas::set. y: {} >= self.height: {}", y, self.height);

        unsafe {
            *self.data.add(x + self.width * y) = pxl;
        }
    }
}

fn draw_line(canvas: &mut Canvas, (mut x0, mut y0): (isize, isize), (mut x1, mut y1): (isize, isize)) {
    // TODO: bresenhams algorithm
    if (x1 - x0).abs() > (y1 - y0).abs() {
        // line more horizontal than vertical

        if x0 > x1 {
            swap(&mut x0, &mut x1);
            swap(&mut y0, &mut y1);
        }

        for (x, y) in (x0..=x1).zip(interpolate((x0, y0), (x1, y1)).into_iter()) {
            canvas.set((x as usize, y as usize), Pixel { r: 255, g: 255, b: 255, a: 255 });
        }
    } else {
        // line more vertical than horizontal

        if y0 > y1 {
            swap(&mut x0, &mut x1);
            swap(&mut y0, &mut y1);
        }

        for (y, x) in (y0..=y1).zip(interpolate((y0, x0), (y1, x1)).into_iter()) {
            canvas.set((x as usize, y as usize), Pixel { r: 255, g: 255, b: 255, a: 255 });
        }
    }

    fn interpolate((i0, d0): (isize, isize), (i1, d1): (isize, isize)) -> Vec<isize> {
        if i0 == i1 {
            return vec![d0];
        }

        let mut values = Vec::new();

        let a = (d1 - d0) as f64 / (i1 - i0) as f64;
        let mut d = d0 as f64;
        for _i in i0..=i1 {
            values.push(d as isize);
            d += a;
        }

        values
    }
}

fn draw_str(
    canvas: &mut Canvas,
    s: &str,
    font: &rusttype::Font,
    scale: rusttype::Scale,
    start: rusttype::Point<f32>
) {
    let vmetrics = font.v_metrics(scale);
    for g in font.layout(s, scale, start) {
        if let Some(bbox) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                let n = (255.0 * v) as u8;
                canvas.set(
                    (
                        (bbox.min.x as f32 + x as f32) as usize,
                        (vmetrics.ascent + bbox.min.y as f32 + y as f32) as usize
                    ),
                    Pixel { r: n, g: n, b: n, a: n }
                );
            });
        }
    }
}

/// Message dispatch loop. Dispatches all messages in queue.
///
/// Returns `true`, unless WM_QUIT was received.
pub fn dispatch_messages() -> bool {
    use winapi::um::winuser::{
        DispatchMessageA,
        PeekMessageA,
        TranslateMessage,
        PM_REMOVE,
        WM_QUIT,
    };

    loop {
        let msg = unsafe {
            let mut msg = std::mem::MaybeUninit::uninit();
            if PeekMessageA(msg.as_mut_ptr(), std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                Some(msg.assume_init())
            } else {
                None
            }
        };

        match msg {
            None => break true,
            Some(msg) if msg.message == WM_QUIT => break false,
            Some(msg) => unsafe {
                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            },
        }
    }
}

fn stretch_di_bits_win_except(
    device_context: HDC,
    width: c_int,
    height: c_int,
    canvas: &Canvas,
    bitmap_info: &BITMAPINFO,
) {
    use winapi::um::wingdi::{
        StretchDIBits,
        DIB_RGB_COLORS,
        SRCCOPY,
    };
    win_except(
        unsafe { StretchDIBits(
            device_context,
            0,
            0,
            width,
            height,
            0,
            0,
            canvas.width as _,
            canvas.height as _,
            canvas.data as *mut _,
            bitmap_info,
            DIB_RGB_COLORS,
            SRCCOPY,
        ) },
        format!(
                "
    StretchDIBits failed.
    StretchDIBits (
        hdc: {:p},
        xDest: {},
        yDest: {},
        DestWidth: {},
        DestHeight: {},
        xSrc: {},
        ySrc: {},
        SrcWidth: {},
        SrcHeight: {},
        lpBits: {:p},
        lpbmi: {:p},
        iUsage: {},
        rop: {},
    )",
            device_context,
            0,
            0,
            width,
            height,
            0,
            0,
            canvas.width,
            canvas.height,
            canvas.data,
            &bitmap_info,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    );
}

unsafe extern "system" fn window_procedure (hwnd: HWND, u_msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    DefWindowProcA (hwnd, u_msg, w_param, l_param)
}