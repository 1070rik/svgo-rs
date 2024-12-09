use std::io;
use quick_xml::events::BytesStart;

/// Trait that must be implemented by all SVG optimization plugins.
///
/// This trait defines the lifecycle and processing capabilities of a plugin:
/// - `init`: Called before processing begins
/// - `process_element`: Called for each XML element
/// - `finalize`: Called after all elements have been processed
/// - `name`: Returns the plugin's name for identification
pub trait SVGPlugin: PluginStatistics {
    /// Initialize the plugin before processing begins.
    ///
    /// This method is called once before processing the SVG file.
    /// Use this to set up any state the plugin needs.
    fn init(&mut self) -> io::Result<()> {
        Ok(()) // Default no-op implementation
    }

    /// Process a single XML element.
    ///
    /// This method is called for each element in the SVG file.
    /// The element can be modified in-place to perform optimizations.
    ///
    /// # Arguments
    /// * `element` - Mutable reference to the XML element being processed
    ///
    /// # Returns
    /// * `io::Result<()>` - Success or error during processing
    fn process_element(&mut self, element: &mut BytesStart) -> io::Result<()>;

    /// Finalize processing and clean up.
    ///
    /// This method is called after all elements have been processed.
    /// Use this to clean up resources and return final statistics.
    fn finalize(&mut self) -> io::Result<()> {
        Ok(()) // Default no-op implementation
    }

    /// Get the name of the plugin.
    ///
    /// This is used for logging and user feedback.
    fn name(&self) -> &str;
}

/// Trait for plugins that can provide optimization statistics.
pub trait PluginStatistics {
    /// Get human-readable statistics about the optimizations performed.
    fn get_statistics(&self) -> Vec<(&str, String)>;
}

/// Trait for plugins that support configuration.
pub trait ConfigurablePlugin {
    /// The configuration type for this plugin.
    type Config;

    /// Configure the plugin with the given settings.
    ///
    /// # Arguments
    /// * `config` - Configuration settings for the plugin
    fn configure(&mut self, config: Self::Config) -> io::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::BytesStart;

    // Example minimal plugin for testing
    struct TestPlugin;

    impl SVGPlugin for TestPlugin {
        fn process_element(&mut self, _element: &mut BytesStart) -> io::Result<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            "TestPlugin"
        }
    }

    impl PluginStatistics for TestPlugin {
        fn get_statistics(&self) -> Vec<(&str, String)> {
            Vec::new()
        }
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut plugin = TestPlugin;

        // Test default implementations
        assert!(plugin.init().is_ok());
        assert!(plugin.finalize().is_ok());

        // Test name
        assert_eq!(plugin.name(), "TestPlugin");

        // Test process_element with dummy element
        let mut element = BytesStart::new("test");
        assert!(plugin.process_element(&mut element).is_ok());
    }
}
