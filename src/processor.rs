use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use crate::cli::PluginConfig;
use crate::plugins::{
    PathOptimizerPlugin,
    // DeduplicateGradientsPlugin,
    // RemoveIDPlugin,
    // RemoveDataAttributesPlugin,
    SVGPlugin,
};

pub struct SVGProcessor {
    chunk_size: usize,
    plugins: Vec<Box<dyn SVGPlugin>>,
    start_time: Option<Instant>,
    processing_time: Option<f64>,
}

impl SVGProcessor {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            plugins: Vec::new(),
            start_time: None,
            processing_time: None,
        }
    }

    pub fn add_plugin<P: SVGPlugin + 'static>(&mut self, plugin: P) {
        self.plugins.push(Box::new(plugin));
    }

    pub fn process_file<P: AsRef<Path>>(
        &mut self,
        input_path: P,
        output_path: P,
    ) -> io::Result<()> {
        self.start_time = Some(Instant::now());

        // Initialize all plugins
        for plugin in &mut self.plugins {
            plugin.init()?;
        }

        let input_file = File::open(input_path)?;
        let output_file = File::create(output_path)?;

        let buf_reader = BufReader::with_capacity(self.chunk_size, input_file);
        let buf_writer = BufWriter::with_capacity(self.chunk_size, output_file);

        let mut reader = Reader::from_reader(buf_reader);
        let mut writer = Writer::new(buf_writer);

        let mut xml_buf = Vec::with_capacity(self.chunk_size);
        let mut processed = false;
        let mut bytes_processed = 0;
        let process_start = Instant::now();

        loop {
            match reader.read_event_into(&mut xml_buf) {
                Ok(Event::Eof) => break,
                Ok(event) => {
                    let processed_event = self.process_event(event)?;
                    writer.write_event(processed_event)?;
                    processed = true;
                    xml_buf.clear();
                }
                Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
            }
        }

        // Finalize all plugins
        for plugin in &mut self.plugins {
            plugin.finalize()?;
        }

        // Ensure all data is written
        let mut inner = writer.into_inner();
        inner.flush()?;

        if !processed {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No SVG content was processed",
            ));
        }

        let process_duration = process_start.elapsed().as_secs_f64();
        self.processing_time = Some(process_duration);

        // Calculate processing speed
        let speed_mb_per_sec = (bytes_processed as f64 / 1_048_576.0) / process_duration;

        // Store timing information for later retrieval
        self.processing_time = Some(process_duration);

        Ok(())
    }

    fn process_event<'a>(&mut self, mut event: Event<'a>) -> io::Result<Event<'a>> {
        match &mut event {
            Event::Start(elem) | Event::Empty(elem) => {
                // Apply all plugins to the element
                for plugin in &mut self.plugins {
                    plugin.process_element(elem)?;
                }
            }
            _ => {}
        }

        // Return the modified event
        Ok(event)
    }

    pub fn get_statistics(&self) -> ProcessingStatistics {
        ProcessingStatistics {
            processing_time: self.processing_time,
            total_time: self.start_time.map(|t| t.elapsed().as_secs_f64()),
        }
    }
}

pub struct ProcessingStatistics {
    pub processing_time: Option<f64>,
    pub total_time: Option<f64>,
}

pub struct SVGProcessorCLI {
    processor: SVGProcessor,
    verbose: bool,
}

impl SVGProcessorCLI {
    pub fn new(buffer_size: usize, verbose: bool) -> Self {
        Self {
            processor: SVGProcessor::new(buffer_size * 1024), // Convert KB to bytes
            verbose,
        }
    }

    pub fn configure(&mut self, config: PluginConfig) -> &mut Self {
        if let Some(path_config) = config.path_optimizer {
            if self.verbose {
                println!(
                    "Enabling path optimizer with {} decimal places",
                    path_config.decimal_places
                );
            }
            self.processor
                .add_plugin(PathOptimizerPlugin::new(path_config.decimal_places));
        }

        if config.gradient_deduplicator {
            if self.verbose {
                println!("Enabling gradient deduplicator");
            }
            // self.processor.add_plugin(DeduplicateGradientsPlugin::new());
        }

        if config.id_remover.enabled {
            if self.verbose {
                println!("Enabling ID remover");
                if !config.id_remover.preserve.is_empty() {
                    println!("Preserving IDs: {:?}", config.id_remover.preserve);
                }
            }
            // self.processor.add_plugin(RemoveIDPlugin::new(
            //     config.id_remover.preserve
            // ));
        }

        if config.data_attr_remover {
            if self.verbose {
                println!("Enabling data attribute remover");
            }
            // self.processor.add_plugin(RemoveDataAttributesPlugin);
        }

        self
    }

    pub fn process<P: AsRef<Path>>(&mut self, input: P, output: P) -> io::Result<()> {
        if self.verbose {
            println!(
                "Processing {} -> {}",
                input.as_ref().display(),
                output.as_ref().display()
            );
        }

        let start = Instant::now();
        let result = self.processor.process_file(input, output);

        if let Err(ref e) = result {
            if self.verbose {
                eprintln!("Error during processing: {}", e);
            }
        } else if self.verbose {
            let stats = self.processor.get_statistics();
            println!("\nProcessing Statistics:");
            println!("--------------------");

            if let Some(processing_time) = stats.processing_time {
                println!("Processing time: {:.2} seconds", processing_time);
            }

            if let Some(total_time) = stats.total_time {
                println!("Total time: {:.2} seconds", total_time);
            }

            println!("--------------------");

            // Loop over all plugins and call the get_statistics method
            for plugin in &self.processor.plugins {
                println!("\n{} Statistics:", plugin.name());
                println!("--------------------");
                let plugin_stats = plugin.get_statistics();
                for (name, value) in plugin_stats {
                    println!("{}: {}", name, value);
                }
                println!("--------------------");
            }
        }

        result
    }

    pub fn list_plugins() {
        println!("Available plugins:");
        println!("  1. Path Optimizer");
        println!("     --optimize-paths");
        println!("     --path-decimals <VALUE>");
        println!(
            "     Optimizes path data by reducing decimal places and removing unnecessary spaces"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_optimization() -> io::Result<()> {
        let mut processor = SVGProcessor::new(1024);
        processor.add_plugin(PathOptimizerPlugin::new(2));

        // Create test input file
        let test_svg = r#"<?xml version="1.0"?>
            <svg>
                <path d="M 100.000 200.000 L 300.000 400.000"/>
            </svg>"#;

        let temp_dir = tempfile::tempdir()?;
        let input_path = temp_dir.path().join("input.svg");
        let output_path = temp_dir.path().join("output.svg");

        std::fs::write(&input_path, test_svg)?;

        // Process the file
        processor.process_file(input_path, output_path.clone())?;

        // Read and verify output
        let output_content = std::fs::read_to_string(output_path)?;
        assert!(output_content.contains(r#"d="M100 200L300 400""#));

        Ok(())
    }
}
