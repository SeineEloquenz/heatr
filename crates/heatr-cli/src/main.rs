//! Heatr command-line interface.

use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use heatr::{Api, Duration, Generation, HeatingStatus, Preferences, SkinSensitivity};
use std::time::Duration as StdDuration;

#[derive(Parser)]
#[command(
    name = "heatr",
    version,
    about = "Tech demo for interfacing with heat-based USB insect bite healers",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show a list of connected bite healers.
    Info,
    /// Run the initialization sequence on a connected bite healer.
    ///
    /// Must be run once after connecting the device before the first `start`.
    Init,
    /// Activate a connected bite healer for demonstration purposes.
    ///
    /// Requires `init` to have been run first in this device session.
    Start {
        /// Duration of the session.
        #[arg(long, value_enum, default_value_t = CliDuration::Short)]
        duration: CliDuration,
        /// Target generation.
        #[arg(long, value_enum, default_value_t = CliGeneration::Child)]
        generation: CliGeneration,
        /// Skin sensitivity setting.
        #[arg(long, value_enum, default_value_t = CliSkinSensitivity::Sensitive)]
        skin_sensitivity: CliSkinSensitivity,
    },
}

#[derive(Clone, ValueEnum)]
enum CliDuration {
    Short,
    Medium,
    Long,
}

impl From<CliDuration> for Duration {
    fn from(d: CliDuration) -> Self {
        match d {
            CliDuration::Short => Duration::Short,
            CliDuration::Medium => Duration::Medium,
            CliDuration::Long => Duration::Long,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum CliGeneration {
    Child,
    Adult,
}

impl From<CliGeneration> for Generation {
    fn from(g: CliGeneration) -> Self {
        match g {
            CliGeneration::Child => Generation::Child,
            CliGeneration::Adult => Generation::Adult,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum CliSkinSensitivity {
    Sensitive,
    Regular,
}

impl From<CliSkinSensitivity> for SkinSensitivity {
    fn from(s: CliSkinSensitivity) -> Self {
        match s {
            CliSkinSensitivity::Sensitive => SkinSensitivity::Sensitive,
            CliSkinSensitivity::Regular => SkinSensitivity::Regular,
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let cli = Cli::parse();
    let api = Api::new();

    let result = match cli.command {
        Commands::Init => api.init(),
        Commands::Info => api.info().map(|healers| {
            if healers.is_empty() {
                println!("No known bite healers detected.");
            } else {
                println!("{} bite healer(s) detected:", healers.len());
                for h in &healers {
                    let status = if h.supported() {
                        "supported"
                    } else {
                        "unsupported"
                    };
                    let product = h.usb_product_name.as_deref().unwrap_or("(name unknown)");
                    let serial = h.serial_number.as_deref().unwrap_or("unknown S/N");
                    println!(
                        "  [{}] {} – {} (USB: {}, S/N: {}, vendor: {})",
                        status,
                        h.product_name(),
                        h.support_statement.comment.unwrap_or(""),
                        product,
                        serial,
                        h.vendor_name(),
                    );
                }
            }
        }),
        Commands::Start {
            duration,
            generation,
            skin_sensitivity,
        } => {
            let preferences = Preferences {
                duration: duration.into(),
                generation: generation.into(),
                skin_sensitivity: skin_sensitivity.into(),
            };
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}  [{elapsed_precise}]")
                    .unwrap(),
            );
            pb.enable_steady_tick(StdDuration::from_millis(80));
            pb.set_message("Heating…");
            let mut heated = false;
            let result = api.start(preferences, |status: &HeatingStatus| {
                if status.is_heating {
                    pb.set_message(format!("Heating    temp {:3}/225", status.temperature));
                } else {
                    if !heated {
                        pb.println("  Heated.");
                        heated = true;
                    }
                    pb.set_message(format!("Applying   temp {:3}/225", status.temperature));
                }
            });
            match result {
                Ok(()) => {
                    pb.finish_with_message("Treatment complete.");
                    Ok(())
                }
                Err(e) => {
                    pb.abandon_with_message("Failed.");
                    Err(e)
                }
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
