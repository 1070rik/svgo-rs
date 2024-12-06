use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "svgo-rs")]
#[command(about = "SVG optimization tool written in Rust", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Buffer size in KB for reading/writing files
    #[arg(short, long, default_value = "8")]
    pub buffer_size: usize,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Optimize SVG files
    Optimize(OptimizeArgs),

    /// List available plugins
    ListPlugins,

    /// Show optimization statistics for an SVG file
    Analyze(AnalyzeArgs),
}

#[derive(Args)]
pub struct OptimizeArgs {
    /// Input SVG file
    #[arg(required = true)]
    pub input: PathBuf,

    /// Output SVG file
    #[arg(required = true)]
    pub output: PathBuf,

    /// Enable path optimization
    #[arg(long)]
    pub optimize_paths: bool,

    /// Decimal places for path optimization (default: 2)
    #[arg(long, default_value = "2")]
    pub path_decimals: usize,

    /// Enable gradient deduplication
    #[arg(long)]
    pub dedupe_gradients: bool,

    /// Enable ID removal
    #[arg(long)]
    pub remove_ids: bool,

    /// Enable data attribute removal
    #[arg(long)]
    pub remove_data_attrs: bool,

    /// Preserve specified IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub preserve_ids: Option<Vec<String>>,
}

#[derive(Args)]
pub struct AnalyzeArgs {
    /// Input SVG file
    #[arg(required = true)]
    pub input: PathBuf,
}

// Plugin configuration structures
#[derive(Default)]
pub struct PluginConfig {
    pub path_optimizer: Option<PathOptimizerConfig>,
    pub gradient_deduplicator: bool,
    pub id_remover: IdRemoverConfig,
    pub data_attr_remover: bool,
}

pub struct PathOptimizerConfig {
    pub decimal_places: usize,
}

pub struct IdRemoverConfig {
    pub enabled: bool,
    pub preserve: Vec<String>,
}

impl Default for IdRemoverConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            preserve: Vec::new(),
        }
    }
}

impl From<&OptimizeArgs> for PluginConfig {
    fn from(args: &OptimizeArgs) -> Self {
        Self {
            path_optimizer: if args.optimize_paths {
                Some(PathOptimizerConfig {
                    decimal_places: args.path_decimals,
                })
            } else {
                None
            },
            gradient_deduplicator: args.dedupe_gradients,
            id_remover: IdRemoverConfig {
                enabled: args.remove_ids,
                preserve: args.preserve_ids.clone().unwrap_or_default(),
            },
            data_attr_remover: args.remove_data_attrs,
        }
    }
}
