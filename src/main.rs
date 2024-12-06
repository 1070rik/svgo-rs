mod cli;
mod processor;
mod plugins;

use std::process;
use clap::Parser;
use cli::{Cli, Commands, OptimizeArgs, PluginConfig};
use processor::SVGProcessorCLI;

fn run(cli: Cli) -> std::io::Result<()> {
    match cli.command {
        Commands::ListPlugins => {
            SVGProcessorCLI::list_plugins();
            Ok(())
        },

        Commands::Optimize(args) => {
            let mut processor = SVGProcessorCLI::new(
                cli.buffer_size,
                cli.verbose
            );

            // Convert OptimizeArgs into PluginConfig
            let config = PluginConfig::from(&args);

            // Configure and run the processor
            processor
                .configure(config)
                .process(&args.input, &args.output)
        },

        Commands::Analyze(args) => {
            if cli.verbose {
                println!("Analyzing SVG file: {}", args.input.display());
            }

            // Create processor with all plugins enabled for analysis
            let mut processor = SVGProcessorCLI::new(cli.buffer_size, true);
            let config = PluginConfig {
                path_optimizer: Some(cli::PathOptimizerConfig {
                    decimal_places: 2,
                }),
                gradient_deduplicator: true,
                id_remover: cli::IdRemoverConfig::default(),
                data_attr_remover: true,
            };

            // Create temporary output path for analysis
            let temp_output = args.input.with_extension("analysis.svg");

            processor
                .configure(config)
                .process(&args.input, &temp_output)?;

            // Clean up temporary file
            std::fs::remove_file(temp_output)?;

            Ok(())
        }
    }
}

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    // Run the application
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_optimize_args_to_config() {
        let args = OptimizeArgs {
            input: PathBuf::from("input.svg"),
            output: PathBuf::from("output.svg"),
            optimize_paths: true,
            path_decimals: 3,
            dedupe_gradients: true,
            remove_ids: true,
            remove_data_attrs: false,
            preserve_ids: Some(vec!["id1".to_string(), "id2".to_string()]),
        };

        let config = PluginConfig::from(&args);

        assert!(config.path_optimizer.is_some());
        assert_eq!(config.path_optimizer.unwrap().decimal_places, 3);
        assert!(config.gradient_deduplicator);
        assert!(config.id_remover.enabled);
        assert!(!config.data_attr_remover);
        assert_eq!(config.id_remover.preserve.len(), 2);
    }
}
