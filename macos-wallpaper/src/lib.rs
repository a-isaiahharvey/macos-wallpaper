use std::{borrow::Borrow, thread::sleep, time::Duration};

use icrate::{
    objc2::{
        rc::{Id, Shared},
        runtime::Object,
    },
    AppKit::{
        NSColor, NSImageScaleAxesIndependently, NSImageScaleProportionallyUpOrDown, NSScreen,
        NSWorkspace, NSWorkspaceDesktopImageAllowClippingKey, NSWorkspaceDesktopImageFillColorKey,
        NSWorkspaceDesktopImageScalingKey,
    },
    Foundation::{
        NSApplicationSupportDirectory, NSArray, NSDictionary, NSError, NSFileManager,
        NSMutableDictionary, NSNumber, NSString, NSUserDomainMask, NSURL,
    },
};
use inline_python::python;

#[derive(Debug, Clone)]
pub enum Screen {
    All,
    Main,
    Index(usize),
    NSScreen(Vec<Id<NSScreen, Shared>>),
}

#[derive(Debug, Clone)]

pub enum Scale {
    Auto,
    Fill,
    Fit,
    Stretch,
    Center,
}

impl Screen {
    pub fn nsscreens(&self) -> Id<NSArray<NSScreen>, Shared> {
        unsafe {
            match self {
                Screen::All => NSScreen::screens(),
                Screen::Main => {
                    if let Some(main_screen) = NSScreen::mainScreen() {
                        NSArray::from_slice(&[main_screen])
                    } else {
                        NSArray::array()
                    }
                }
                Screen::Index(index) => {
                    if NSScreen::screens().count() > *index {
                        let screen = NSScreen::screens().objectAtIndex(*index);
                        NSArray::from_slice(&[screen])
                    } else {
                        NSArray::array()
                    }
                }
                Screen::NSScreen(screens) => NSArray::from_vec(screens.to_vec()),
            }
        }
    }
}

pub fn get_from_directory(url: &NSURL) -> Option<Id<NSURL, Shared>> {
    unsafe {
        let py_context = inline_python::Context::new();
        let app_support_directory = NSFileManager::defaultManager()
            .URLForDirectory_inDomain_appropriateForURL_create_error(
                NSApplicationSupportDirectory,
                NSUserDomainMask,
                None,
                false,
            )
            .ok()?;

        let db_url = app_support_directory.URLByAppendingPathComponent_isDirectory(
            &NSString::from_str("Dock/desktoppicture.db"),
            false,
        )?;

        let db_path = db_url.path()?.to_string();
        let db_path = db_path.as_str();

        py_context.run(python! {
            // Import the sqlite3 package
            import sqlite3

            // Connect to the wallpaper database
            conn = sqlite3.connect('db_path)
            c = conn.cursor()
        });

        let table = "data";
        let column = "value";
        let row_id = "rowid";

        let max_id: isize = {
            py_context.run(python! {
                c.execute("SELECT {} FROM {} ORDER BY {} DESC LIMIT 1".format('row_id, 'table, 'row_id))
                data = c.fetchall()

                result = data[0][0]
            });

            py_context.get("result")
        };

        let image: String = {
            py_context.run(python! {
                c.execute("SELECT {} FROM {} WHERE {} == {}".format('column, 'table, 'row_id, 'max_id))
                data = c.fetchall()

                result = data[0][0]
            });

            py_context.get("result")
        };

        py_context.run(python! {
            c.close()
            conn.close()
        });

        url.URLByAppendingPathComponent_isDirectory(&NSString::from_str(&image), false)
    }
}

/// Get the current wallpapers.
pub fn get_current(screen: Option<&Screen>) -> Vec<Id<NSURL, Shared>> {
    unsafe {
        let screen = match screen {
            Some(sceen) => sceen,
            None => &Screen::All,
        };

        let wallpaper_urls = screen
            .nsscreens()
            .iter()
            .filter_map(|screen| NSWorkspace::sharedWorkspace().desktopImageURLForScreen(screen))
            .collect::<Vec<Id<NSURL, Shared>>>();

        wallpaper_urls
            .iter()
            .filter_map(|url| {
                if url.hasDirectoryPath() {
                    get_from_directory(url)
                } else {
                    Some(url.to_owned())
                }
            })
            .collect::<Vec<Id<NSURL, Shared>>>()
    }
}

fn force_refresh_if_needed(image: &NSURL, screen: &Screen) -> Result<(), Id<NSError, Shared>> {
    let mut should_sleep = false;
    let current_images = get_current(Some(screen));

    for (index, screen) in screen.nsscreens().iter().enumerate() {
        unsafe {
            if image == current_images[index].borrow() {
                should_sleep = true;
                NSWorkspace::sharedWorkspace().setDesktopImageURL_forScreen_options_error(
                    &NSURL::fileURLWithPath(&NSString::from_str("./")),
                    screen,
                    &NSDictionary::dictionary(),
                )?;
            }
        }
    }

    if should_sleep {
        sleep(Duration::from_secs_f32(0.4));
    }

    Ok(())
}

/// Set an image URL as wallpaper.
pub fn set_image(
    image: &NSURL,
    screen: Option<&Screen>,
    scale: Option<Scale>,
    fill_color: Option<&NSColor>,
) -> Result<(), Id<NSError, Shared>> {
    unsafe {
        let options = NSMutableDictionary::<NSString, Object>::dictionary();

        let screen = match screen {
            Some(value) => value,
            None => &Screen::All,
        };

        let scale = match scale {
            Some(value) => value,
            None => Scale::Auto,
        };

        match scale {
            Scale::Auto => (),
            Scale::Fill => {
                options.setObject_forKey(
                    &NSNumber::numberWithUnsignedInteger(NSImageScaleProportionallyUpOrDown),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setObject_forKey(
                    &NSNumber::numberWithBool(true),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
            Scale::Fit => {
                options.setObject_forKey(
                    &NSNumber::numberWithUnsignedInteger(NSImageScaleProportionallyUpOrDown),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setObject_forKey(
                    &NSNumber::numberWithBool(false),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
            Scale::Stretch => {
                options.setObject_forKey(
                    &NSNumber::numberWithUnsignedInteger(NSImageScaleAxesIndependently),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setObject_forKey(
                    &NSNumber::numberWithBool(true),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
            Scale::Center => {
                options.setObject_forKey(
                    &NSNumber::numberWithUnsignedInteger(NSImageScaleProportionallyUpOrDown),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setObject_forKey(
                    &NSNumber::numberWithBool(false),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
        }

        options.setObject_forKey(
            fill_color.unwrap_or(&NSColor::clearColor()),
            &NSString::stringWithString(NSWorkspaceDesktopImageFillColorKey),
        );

        force_refresh_if_needed(image, screen)?;

        for screen in screen.nsscreens().iter() {
            NSWorkspace::sharedWorkspace()
                .setDesktopImageURL_forScreen_options_error(image, screen, &options)?
        }

        Ok(())
    }
}

/// Set a solid color as wallpaper.
pub fn set_color(color: &NSColor, screen: Option<&Screen>) -> Result<(), Id<NSError, Shared>> {
    unsafe {
        let transparent_image = NSURL::fileURLWithPath(&NSString::from_str("/System/Library/PreferencePanes/DesktopScreenEffectsPref.prefPane/Contents/Resources/DesktopPictures.prefPane/Contents/Resources/Transparent.tiff"));

        set_image(&transparent_image, screen, Some(Scale::Fit), Some(color))
    }
}

/// Names of available screens.
pub fn screen_names() -> Vec<String> {
    unsafe {
        NSScreen::screens()
            .iter()
            .map(|screen| screen.localizedName().to_string())
            .collect()
    }
}
