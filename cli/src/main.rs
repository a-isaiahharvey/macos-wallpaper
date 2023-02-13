use clap::{Parser, Subcommand};
use icrate::{
    objc2::rc::{Id, Shared},
    Foundation::{NSError, NSString, NSURL},
};
use macos_wallpaper::{get_current, set_color, set_image};
use utils::{nscolor_from_hex, nscolor_from_rgb, screen_from_str};

mod utils;

#[derive(Parser, Debug)]
#[command(name = "wallpaper-cli")]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Get current wallpaper images.
    GetCurrent {
        #[arg(default_value_t = String::from("main"))]
        screen: String,
    },

    /// Set image as background
    SetImage {
        /// Path to image
        path: String,
    },

    /// Set solid color as wallpaper from hex value.
    SetHexColor {
        /// Color represented as hex Eg. #bcc6b2
        color: String,
    },

    /// Set solid color as wallpaper from RGB value.
    SetRgbColor {
        /// Red value
        r: u8,
        /// Green value
        g: u8,
        /// Blue value
        b: u8,
    },
}

#[tokio::main]
async fn main() -> Result<(), Id<NSError, Shared>> {
    let args = Args::parse();

    match args.command {
        Commands::GetCurrent { screen } => {
            let wallpapers = get_current(screen_from_str(&screen).as_ref()).await;

            for wallpaper in wallpapers {
                if let Some(path) = unsafe { wallpaper.path() } {
                    println!("{path}")
                }
            }
        }

        Commands::SetImage { path } => unsafe {
            let image = NSURL::fileURLWithPath(&NSString::from_str(&path));

            set_image(&image, None, None, None).await?;
        },

        Commands::SetHexColor { color } => {
            if let Some(color) = nscolor_from_hex(&color) {
                set_color(&color, None).await?;
            }
        }

        Commands::SetRgbColor { r, g, b } => {
            if let Some(color) = nscolor_from_rgb(r, g, b) {
                set_color(&color, None).await?;
            }
        }
    }

    Ok(())
}
