use std::{borrow::Borrow, thread::sleep, time::Duration};

use objc2::{rc::Id, runtime::AnyObject};
use objc2_app_kit::{
    NSColor, NSImageScaling, NSScreen, NSWorkspace, NSWorkspaceDesktopImageAllowClippingKey,
    NSWorkspaceDesktopImageFillColorKey, NSWorkspaceDesktopImageScalingKey,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSDictionary, NSError, NSFileManager, NSMutableDictionary, NSNumber,
    NSSearchPathDirectory, NSSearchPathDomainMask, NSString, NSURL,
};

#[derive(Debug, Clone)]
pub enum Screen {
    All,
    Main,
    Index(usize),
    NSScreen(Vec<Id<NSScreen>>),
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
    pub fn nsscreens(&self) -> Id<NSArray<NSScreen>> {
        unsafe {
            let mtm: MainThreadMarker = MainThreadMarker::new_unchecked();

            match self {
                Screen::All => NSScreen::screens(mtm),
                Screen::Main => {
                    if let Some(main_screen) = NSScreen::mainScreen(mtm) {
                        NSArray::from_slice(&[&main_screen])
                    } else {
                        NSArray::array()
                    }
                }
                Screen::Index(index) => {
                    if NSScreen::screens(mtm).count() > *index {
                        let screen = NSScreen::screens(mtm).objectAtIndex(*index);
                        NSArray::from_slice(&[&screen])
                    } else {
                        NSArray::array()
                    }
                }
                Screen::NSScreen(screens) => NSArray::from_vec(screens.to_vec()),
            }
        }
    }
}

pub async fn get_from_directory(url: &NSURL) -> Option<Id<NSURL>> {
    unsafe {
        let app_support_directory = NSFileManager::defaultManager()
            .URLForDirectory_inDomain_appropriateForURL_create_error(
                NSSearchPathDirectory::NSApplicationSupportDirectory,
                NSSearchPathDomainMask::NSUserDomainMask,
                None,
                false,
            )
            .ok()?;

        let db_url = app_support_directory.URLByAppendingPathComponent_isDirectory(
            &NSString::from_str("Dock/desktoppicture.db"),
            false,
        )?;

        let db_path = db_url.path()?.to_string();

        let conn = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&db_path)
            .await
            .ok()?;

        use sqlx::FromRow;

        let max_id = {
            #[derive(Clone, FromRow, Debug)]
            struct DbRow {
                rowid: i32,
            }
            let row =
                sqlx::query_as::<_, DbRow>("SELECT rowid FROM data ORDER BY rowid DESC LIMIT 1")
                    .fetch_one(&conn)
                    .await
                    .unwrap();
            row.rowid
        };

        let image: String = {
            #[derive(Clone, FromRow, Debug)]
            struct DbRow {
                value: String,
            }

            let row = sqlx::query_as::<_, DbRow>("SELECT value FROM data WHERE rowid == ?")
                .bind(max_id)
                .fetch_one(&conn)
                .await
                .unwrap();

            row.value
        };

        url.URLByAppendingPathComponent_isDirectory(&NSString::from_str(&image), false)
    }
}

/// Get the current wallpapers.
pub async fn get_current(screen: Option<&Screen>) -> Vec<Id<NSURL>> {
    unsafe {
        let screen = match screen {
            Some(sceen) => sceen,
            None => &Screen::All,
        };

        let wallpaper_urls = screen
            .nsscreens()
            .iter()
            .filter_map(|screen| NSWorkspace::sharedWorkspace().desktopImageURLForScreen(screen))
            .collect::<Vec<Id<NSURL>>>();

        let mut urls = Vec::new();
        for url in &wallpaper_urls {
            if url.hasDirectoryPath() {
                if let Some(url) = get_from_directory(url).await {
                    urls.push(url)
                }
            } else {
                urls.push(url.to_owned());
            }
        }

        urls
    }
}

async fn force_refresh_if_needed(image: &NSURL, screen: &Screen) -> Result<(), Id<NSError>> {
    let mut should_sleep = false;
    let current_images = get_current(Some(screen)).await;

    for (index, screen) in screen.nsscreens().iter().enumerate() {
        if image == current_images[index].borrow() {
            should_sleep = true;
            unsafe {
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
pub async fn set_image(
    image: &NSURL,
    screen: Option<&Screen>,
    scale: Option<Scale>,
    fill_color: Option<&NSColor>,
) -> Result<(), Id<NSError>> {
    unsafe {
        let mut options = NSMutableDictionary::<NSString, AnyObject>::dictionary();

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
                options.setValue_forKey(
                    Some(&NSNumber::numberWithUnsignedInteger(
                        NSImageScaling::NSImageScaleProportionallyUpOrDown.0,
                    )),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setValue_forKey(
                    Some(&NSNumber::numberWithBool(true)),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
            Scale::Fit => {
                options.setValue_forKey(
                    Some(&NSNumber::numberWithUnsignedInteger(
                        NSImageScaling::NSImageScaleProportionallyUpOrDown.0,
                    )),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setValue_forKey(
                    Some(&NSNumber::numberWithBool(false)),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
            Scale::Stretch => {
                options.setValue_forKey(
                    Some(&NSNumber::numberWithUnsignedInteger(
                        NSImageScaling::NSImageScaleAxesIndependently.0,
                    )),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setValue_forKey(
                    Some(&NSNumber::numberWithBool(true)),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
            Scale::Center => {
                options.setValue_forKey(
                    Some(&NSNumber::numberWithUnsignedInteger(
                        NSImageScaling::NSImageScaleProportionallyUpOrDown.0,
                    )),
                    &NSString::stringWithString(NSWorkspaceDesktopImageScalingKey),
                );
                options.setValue_forKey(
                    Some(&NSNumber::numberWithBool(false)),
                    &NSString::stringWithString(NSWorkspaceDesktopImageAllowClippingKey),
                );
            }
        }

        options.setValue_forKey(
            Some(fill_color.unwrap_or(&NSColor::clearColor())),
            &NSString::stringWithString(NSWorkspaceDesktopImageFillColorKey),
        );

        force_refresh_if_needed(image, screen).await?;

        for screen in screen.nsscreens().iter() {
            NSWorkspace::sharedWorkspace()
                .setDesktopImageURL_forScreen_options_error(image, screen, &options)?
        }

        Ok(())
    }
}

/// Set a solid color as wallpaper.
pub async fn set_color(color: &NSColor, screen: Option<&Screen>) -> Result<(), Id<NSError>> {
    unsafe {
        let transparent_image = NSURL::fileURLWithPath(&NSString::from_str("/System/Library/PreferencePanes/DesktopScreenEffectsPref.prefPane/Contents/Resources/DesktopPictures.prefPane/Contents/Resources/Transparent.tiff"));

        set_image(&transparent_image, screen, Some(Scale::Fit), Some(color)).await
    }
}

/// Names of available screens.
pub fn screen_names() -> Vec<String> {
    unsafe {
        let mtm = MainThreadMarker::new_unchecked();
        NSScreen::screens(mtm)
            .iter()
            .map(|screen| screen.localizedName().to_string())
            .collect()
    }
}
