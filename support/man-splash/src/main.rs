use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const EV_KEY: u16 = 1;
const KEY_LEFTALT: u16 = 56;
const KEY_RIGHTALT: u16 = 100;
const F_SETFL: i32 = 4;
const O_NONBLOCK: i32 = 0x800;

unsafe extern "C" {
    fn fcntl(fd: i32, command: i32, value: i32) -> i32;
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--stop") => {
            let _ = fs::write("/run/man-splash.stop", b"1\n");
        }
        Some("--complete") => {
            let _ = fs::write("/run/man-splash.complete", b"1\n");
        }
        _ => run(),
    }
}

fn run() {
    let _ = fs::remove_file("/run/man-splash.stop");
    // S01 clears stale flags before it launches us.  Do not clear Complete
    // here: S99 may have already requested completion while this process was
    // being scheduled, and removing that request strands the splash forever.
    let _ = fs::remove_file("/run/man-verbose");
    let _ = fs::write("/run/man-splash.pid", format!("{}\n", std::process::id()));

    let mut inputs = input_devices();
    let started = Instant::now();
    let mut input_refresh = Instant::now();
    loop {
        // S01 can run while eudev is still publishing the USB keyboard node.
        // Refresh the complete set so a tablet appearing first cannot hide a
        // keyboard that arrives a few hundred milliseconds later.
        if input_refresh.elapsed() >= Duration::from_millis(500) {
            inputs = input_devices();
            input_refresh = Instant::now();
        }
        if alt_pressed(&mut inputs) {
            enable_verbose();
            break;
        }
        if PathFlag::Stop.exists() {
            break;
        }

        let complete = PathFlag::Complete.exists();
        let elapsed = started.elapsed().as_millis() as u32;
        let progress = if complete {
            100
        } else {
            (8 + elapsed / 85).min(94)
        };
        draw_splash(progress);
        if complete {
            thread::sleep(Duration::from_millis(350));
            break;
        }
        thread::sleep(Duration::from_millis(80));
    }
    let _ = fs::remove_file("/run/man-splash.pid");
}

enum PathFlag {
    Stop,
    Complete,
}
impl PathFlag {
    fn exists(&self) -> bool {
        fs::metadata(match self {
            Self::Stop => "/run/man-splash.stop",
            Self::Complete => "/run/man-splash.complete",
        })
        .is_ok()
    }
}

fn input_devices() -> Vec<File> {
    let Ok(entries) = fs::read_dir("/dev/input") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with("event") {
                return None;
            }
            let file = OpenOptions::new().read(true).open(entry.path()).ok()?;
            unsafe {
                fcntl(file.as_raw_fd(), F_SETFL, O_NONBLOCK);
            }
            Some(file)
        })
        .collect()
}

fn alt_pressed(inputs: &mut [File]) -> bool {
    // Linux input_event is 24 bytes on MAN's 64-bit targets: timeval,
    // type, code, value. Multiple complete events may be returned per read.
    let mut bytes = [0_u8; 24 * 16];
    for input in inputs {
        let Ok(count) = input.read(&mut bytes) else {
            continue;
        };
        for event in bytes[..count].chunks_exact(24) {
            let kind = u16::from_ne_bytes([event[16], event[17]]);
            let code = u16::from_ne_bytes([event[18], event[19]]);
            let value = i32::from_ne_bytes([event[20], event[21], event[22], event[23]]);
            if kind == EV_KEY && (code == KEY_LEFTALT || code == KEY_RIGHTALT) && value > 0 {
                return true;
            }
        }
    }
    false
}

fn enable_verbose() {
    let _ = fs::write("/run/man-verbose", b"1\n");
    let _ = Command::new("dmesg").args(["-n", "8"]).status();
    if let Ok(mut console) = OpenOptions::new().write(true).open("/dev/console") {
        let _ = console.write_all(b"MAN verbose boot: Alt detected\n");
    }
    if let Ok(mut tty) = OpenOptions::new().write(true).open("/dev/tty0") {
        let _ = tty.write_all(b"\x1b[2J\x1b[HMAN verbose boot (Alt detected)\r\n\r\n");
        if let Ok(output) = Command::new("dmesg").output() {
            let _ = tty.write_all(&output.stdout);
        }
    }
}

fn read_u32(path: &str) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn draw_splash(progress: u32) {
    let dimensions = match fs::read_to_string("/sys/class/graphics/fb0/virtual_size") {
        Ok(value) => value,
        Err(_) => return,
    };
    let mut parts = dimensions
        .trim()
        .split(',')
        .filter_map(|part| part.parse::<u32>().ok());
    let (Some(width), Some(height)) = (parts.next(), parts.next()) else {
        return;
    };
    let bpp = read_u32("/sys/class/graphics/fb0/bits_per_pixel").unwrap_or(32);
    if bpp != 32 {
        return;
    }
    let stride = read_u32("/sys/class/graphics/fb0/stride").unwrap_or(width * 4);
    let Ok(mut fb) = OpenOptions::new().write(true).open("/dev/fb0") else {
        return;
    };

    let mut frame = vec![0_u8; (stride * height) as usize];
    fill(&mut frame, stride, 0, 0, width, height, [18, 17, 16, 255]);
    let scale = (width.min(height) / 18).max(12);
    let glyph_y = height.saturating_sub(scale * 5) / 2;
    let glyph_w = scale * 2;
    let gap = scale / 2;
    let total = glyph_w * 3 + gap * 2;
    let glyph_x = width.saturating_sub(total) / 2;
    draw_m(
        &mut frame,
        stride,
        glyph_x,
        glyph_y,
        scale,
        [245, 244, 242, 255],
    );
    draw_a(
        &mut frame,
        stride,
        glyph_x + glyph_w + gap,
        glyph_y,
        scale,
        [245, 244, 242, 255],
    );
    draw_n(
        &mut frame,
        stride,
        glyph_x + (glyph_w + gap) * 2,
        glyph_y,
        scale,
        [245, 244, 242, 255],
    );

    let bar_w = (width * 46 / 100).max(220).min(width.saturating_sub(80));
    let bar_h = (height / 80).clamp(8, 16);
    let bar_x = (width - bar_w) / 2;
    let bar_y = (glyph_y + scale * 4 + height / 12).min(height.saturating_sub(bar_h + 20));
    fill(
        &mut frame,
        stride,
        bar_x,
        bar_y,
        bar_w,
        bar_h,
        [66, 63, 60, 255],
    );
    fill(
        &mut frame,
        stride,
        bar_x,
        bar_y,
        bar_w * progress / 100,
        bar_h,
        [232, 132, 61, 255],
    );
    let _ = fb.write_all(&frame);
}

fn fill(frame: &mut [u8], stride: u32, x: u32, y: u32, w: u32, h: u32, c: [u8; 4]) {
    for py in y..y.saturating_add(h) {
        for px in x..x.saturating_add(w) {
            let at = (py * stride + px * 4) as usize;
            if at + 4 <= frame.len() {
                frame[at..at + 4].copy_from_slice(&c);
            }
        }
    }
}

fn draw_m(f: &mut [u8], s: u32, x: u32, y: u32, z: u32, c: [u8; 4]) {
    fill(f, s, x, y, z / 3, z * 3, c);
    fill(f, s, x + z * 5 / 3, y, z / 3, z * 3, c);
    fill(f, s, x + z / 3, y, z / 3, z, c);
    fill(f, s, x + z * 4 / 3, y, z / 3, z, c);
    fill(f, s, x + z * 2 / 3, y + z / 2, z * 2 / 3, z / 3, c);
}
fn draw_a(f: &mut [u8], s: u32, x: u32, y: u32, z: u32, c: [u8; 4]) {
    fill(f, s, x, y + z / 2, z / 3, z * 5 / 2, c);
    fill(f, s, x + z * 5 / 3, y + z / 2, z / 3, z * 5 / 2, c);
    fill(f, s, x + z / 3, y, z * 4 / 3, z / 3, c);
    fill(f, s, x + z / 3, y + z * 3 / 2, z * 4 / 3, z / 3, c);
}
fn draw_n(f: &mut [u8], s: u32, x: u32, y: u32, z: u32, c: [u8; 4]) {
    fill(f, s, x, y, z / 3, z * 3, c);
    fill(f, s, x + z * 5 / 3, y, z / 3, z * 3, c);
    for i in 0..6 {
        fill(
            f,
            s,
            x + z / 3 + i * z * 2 / 9,
            y + i * z / 2,
            z / 3,
            z / 2,
            c,
        );
    }
}
