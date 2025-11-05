use crate::plugins::traits::{PluginStatistics, SVGPlugin};
use quick_xml::events::BytesStart;
use std::io;

pub struct TransformOptimizerPlugin {
    decimal_places: usize,
    transforms_optimized: usize,
    transforms_removed: usize,
    total_chars_saved: usize,
}

impl TransformOptimizerPlugin {
    pub fn new(decimal_places: usize) -> Self {
        Self {
            decimal_places,
            transforms_optimized: 0,
            transforms_removed: 0,
            total_chars_saved: 0,
        }
    }

    fn optimize_transform(&mut self, transform: &str) -> Option<String> {
        let original_len = transform.len();
        let trimmed = transform.trim();

        if trimmed.is_empty() {
            self.transforms_removed += 1;
            self.total_chars_saved += original_len;
            return None;
        }

        // Parse and optimize transform functions
        let optimized = self.process_transform_functions(trimmed);

        if optimized.is_empty() {
            self.transforms_removed += 1;
            self.total_chars_saved += original_len;
            None
        } else {
            self.transforms_optimized += 1;
            self.total_chars_saved += original_len.saturating_sub(optimized.len());
            Some(optimized)
        }
    }

    fn process_transform_functions(&self, transform: &str) -> String {
        let mut result = Vec::new();
        let mut current_pos = 0;
        let chars: Vec<char> = transform.chars().collect();

        while current_pos < chars.len() {
            // Skip whitespace
            while current_pos < chars.len() && chars[current_pos].is_whitespace() {
                current_pos += 1;
            }

            if current_pos >= chars.len() {
                break;
            }

            // Find function name
            let start = current_pos;
            while current_pos < chars.len() && chars[current_pos].is_alphabetic() {
                current_pos += 1;
            }

            if start == current_pos {
                current_pos += 1;
                continue;
            }

            let func_name: String = chars[start..current_pos].iter().collect();

            // Skip whitespace and find opening parenthesis
            while current_pos < chars.len() && (chars[current_pos].is_whitespace() || chars[current_pos] == '(') {
                if chars[current_pos] == '(' {
                    current_pos += 1;
                    break;
                }
                current_pos += 1;
            }

            // Extract parameters
            let params_start = current_pos;
            let mut paren_depth = 1;
            while current_pos < chars.len() && paren_depth > 0 {
                if chars[current_pos] == '(' {
                    paren_depth += 1;
                } else if chars[current_pos] == ')' {
                    paren_depth -= 1;
                }
                if paren_depth > 0 {
                    current_pos += 1;
                }
            }

            let params_str: String = chars[params_start..current_pos].iter().collect();
            current_pos += 1; // Skip closing parenthesis

            // Parse and optimize parameters
            let params = self.parse_params(&params_str);

            // Check if this is an identity transform
            if self.is_identity_transform(&func_name, &params) {
                continue; // Skip identity transforms
            }

            // Format the optimized transform function
            let optimized_params: Vec<String> = params
                .iter()
                .map(|&p| self.format_number(p))
                .collect();

            result.push(format!("{}({})", func_name, optimized_params.join(" ")));
        }

        result.join(" ")
    }

    fn parse_params(&self, params_str: &str) -> Vec<f64> {
        params_str
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect()
    }

    fn is_identity_transform(&self, func_name: &str, params: &[f64]) -> bool {
        match func_name {
            "translate" | "translateX" | "translateY" => {
                params.iter().all(|&p| p.abs() < 1e-10)
            }
            "scale" => {
                params.iter().all(|&p| (p - 1.0).abs() < 1e-10)
            }
            "rotate" => {
                params.first().map_or(false, |&p| p.abs() < 1e-10)
            }
            "skewX" | "skewY" => {
                params.first().map_or(false, |&p| p.abs() < 1e-10)
            }
            "matrix" => {
                // Identity matrix: matrix(1 0 0 1 0 0)
                if params.len() == 6 {
                    (params[0] - 1.0).abs() < 1e-10
                        && params[1].abs() < 1e-10
                        && params[2].abs() < 1e-10
                        && (params[3] - 1.0).abs() < 1e-10
                        && params[4].abs() < 1e-10
                        && params[5].abs() < 1e-10
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn format_number(&self, num: f64) -> String {
        let formatted = format!("{:.1$}", num, self.decimal_places);
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

impl SVGPlugin for TransformOptimizerPlugin {
    fn init(&mut self) -> io::Result<()> {
        self.transforms_optimized = 0;
        self.transforms_removed = 0;
        self.total_chars_saved = 0;
        Ok(())
    }

    fn process_element(&mut self, element: &mut BytesStart) -> io::Result<()> {
        // Collect and convert all attributes to owned strings
        let mut new_attrs = Vec::new();
        let mut transform_value = None;

        for attr in element.attributes().flatten() {
            if attr.key.as_ref() == b"transform" {
                let data = String::from_utf8_lossy(&attr.value).into_owned();
                transform_value = Some(data);
            } else {
                // Store fully owned strings
                new_attrs.push((
                    String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                    String::from_utf8_lossy(&attr.value).into_owned(),
                ));
            }
        }

        // Process transform if found
        if let Some(transform) = transform_value {
            element.clear_attributes();

            // Add back non-transform attributes
            for (key, value) in new_attrs {
                element.push_attribute((key.as_str(), value.as_str()));
            }

            // Add optimized transform (if not removed)
            if let Some(optimized) = self.optimize_transform(&transform) {
                element.push_attribute(("transform", optimized.as_str()));
            }
        }

        Ok(())
    }

    fn finalize(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "TransformOptimizer"
    }
}

impl PluginStatistics for TransformOptimizerPlugin {
    fn get_statistics(&self) -> Vec<(&str, String)> {
        vec![
            ("Transforms optimized", self.transforms_optimized.to_string()),
            ("Identity transforms removed", self.transforms_removed.to_string()),
            ("Total characters saved", self.total_chars_saved.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_transform_detection() {
        let optimizer = TransformOptimizerPlugin::new(2);

        assert!(optimizer.is_identity_transform("translate", &[0.0, 0.0]));
        assert!(optimizer.is_identity_transform("scale", &[1.0]));
        assert!(optimizer.is_identity_transform("rotate", &[0.0]));
        assert!(optimizer.is_identity_transform("matrix", &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]));

        assert!(!optimizer.is_identity_transform("translate", &[10.0, 20.0]));
        assert!(!optimizer.is_identity_transform("scale", &[2.0]));
        assert!(!optimizer.is_identity_transform("rotate", &[45.0]));
    }

    #[test]
    fn test_number_formatting() {
        let optimizer = TransformOptimizerPlugin::new(2);

        assert_eq!(optimizer.format_number(10.000), "10");
        assert_eq!(optimizer.format_number(10.123), "10.12");
        assert_eq!(optimizer.format_number(0.000), "0");
    }

    #[test]
    fn test_transform_optimization() {
        let mut optimizer = TransformOptimizerPlugin::new(2);

        // Test identity transform removal
        assert_eq!(optimizer.optimize_transform("translate(0, 0)"), None);
        assert_eq!(optimizer.optimize_transform("scale(1)"), None);

        // Test parameter optimization
        let result = optimizer.optimize_transform("translate(10.123, 20.456)");
        assert_eq!(result, Some("translate(10.12 20.46)".to_string()));

        // Test multiple transforms with identity removal
        let result = optimizer.optimize_transform("translate(10, 20) scale(1) rotate(0)");
        assert_eq!(result, Some("translate(10 20)".to_string()));
    }

    #[test]
    fn test_matrix_optimization() {
        let mut optimizer = TransformOptimizerPlugin::new(2);

        // Identity matrix should be removed
        assert_eq!(optimizer.optimize_transform("matrix(1, 0, 0, 1, 0, 0)"), None);

        // Non-identity matrix should be optimized
        let result = optimizer.optimize_transform("matrix(1.5, 0, 0, 1.5, 10.123, 20.456)");
        assert_eq!(result, Some("matrix(1.5 0 0 1.5 10.12 20.46)".to_string()));
    }
}
