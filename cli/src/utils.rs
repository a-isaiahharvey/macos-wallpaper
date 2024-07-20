use std::fmt::Write;

use macos_wallpaper::Screen;
use objc2::rc::Id;
use objc2_app_kit::{NSColor, NSScreen};
use objc2_foundation::{CGFloat, MainThreadMarker};

pub fn nscolor_from_hex(hex: &str) -> Option<Id<NSColor>> {
    let mut result = hex.to_string();

    if let Some(stripped) = hex.strip_prefix('#') {
        result = stripped.to_owned()
    }

    if result.len() == 3 {
        result = result.chars().fold(String::new(), |mut output, c| {
            let _ = write!(output, "{c}{c}");
            output
        });
    }

    let hex = u32::from_str_radix(&result, 16).ok()?;

    unsafe {
        Some(NSColor::colorWithCalibratedRed_green_blue_alpha(
            ((hex >> 16) & 0xFF) as CGFloat / 255.,
            ((hex >> 8) & 0xFF) as CGFloat / 255.,
            (hex & 0xFF) as CGFloat / 255.,
            1.,
        ))
    }
}

pub fn nscolor_from_rgb(r: u8, g: u8, b: u8) -> Option<Id<NSColor>> {
    unsafe {
        Some(NSColor::colorWithCalibratedRed_green_blue_alpha(
            r as CGFloat / 255.,
            g as CGFloat / 255.,
            b as CGFloat / 255.,
            1.,
        ))
    }
}

pub fn screen_from_str(value: &str) -> Option<Screen> {
    match value.trim() {
        "all" => Some(Screen::All),
        "main" => Some(Screen::Main),
        _ => unsafe {
            if let Ok(index) = value.parse::<usize>() {
                let mtm = MainThreadMarker::new_unchecked();
                if index < NSScreen::screens(mtm).len() {
                    return Some(Screen::Index(index));
                }
            }

            None
        },
    }
}
