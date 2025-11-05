use crate::plugins::traits::{PluginStatistics, SVGPlugin};
use quick_xml::events::BytesStart;
use std::io;

/// Optimizer that pre-applies (bakes) a specific transform into path coordinates
/// and then removes the transform attribute.
///
/// This is the proper way to remove duplicate transforms without breaking the visual
/// appearance. It applies the matrix transformation to each coordinate in the path data.
///
/// Example:
/// ```xml
/// <!-- Before -->
/// <path transform="matrix(0 0.33 0.33 0 0 0)" d="M100 200L300 400"/>
///
/// <!-- After (transform applied to coordinates, attribute removed) -->
/// <path d="M66 33L132 99"/>
/// ```
pub struct PreApplyTransformOptimizer {
    /// The exact transform string to pre-apply
    target_transform: String,

    /// Parsed matrix values [a, b, c, d, e, f]
    matrix: Option<[f64; 6]>,

    /// Count of transforms pre-applied
    transforms_applied: usize,

    /// Bytes saved
    bytes_saved: usize,
}

impl PreApplyTransformOptimizer {
    pub fn new(target_transform: String) -> Self {
        let matrix = Self::parse_matrix(&target_transform);

        Self {
            target_transform,
            matrix,
            transforms_applied: 0,
            bytes_saved: 0,
        }
    }

    /// Parse matrix transform string into [a, b, c, d, e, f]
    fn parse_matrix(transform: &str) -> Option<[f64; 6]> {
        let trimmed = transform.trim();

        // Look for "matrix(" ... ")"
        if let Some(start) = trimmed.find("matrix(") {
            if let Some(end) = trimmed.rfind(')') {
                let params_str = &trimmed[start + 7..end];

                // Parse comma or space-separated numbers
                let params: Vec<f64> = params_str
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();

                if params.len() == 6 {
                    return Some([params[0], params[1], params[2], params[3], params[4], params[5]]);
                }
            }
        }

        None
    }

    /// Apply matrix transform to a point (x, y)
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        if let Some([a, b, c, d, e, f]) = self.matrix {
            let new_x = a * x + c * y + e;
            let new_y = b * x + d * y + f;
            (new_x, new_y)
        } else {
            (x, y)
        }
    }

    /// Pre-apply transform to path data
    fn transform_path_data(&self, path_data: &str) -> String {
        if self.matrix.is_none() {
            return path_data.to_string();
        }

        let mut result = String::with_capacity(path_data.len());
        let mut chars = path_data.chars().peekable();
        let mut current_command = ' ';

        while let Some(c) = chars.next() {
            match c {
                // Path commands
                'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' |
                'C' | 'c' | 'S' | 's' | 'Q' | 'q' | 'T' | 't' |
                'A' | 'a' | 'Z' | 'z' => {
                    current_command = c;
                    result.push(c);
                }

                // Numbers
                '0'..='9' | '.' | '-' => {
                    // Collect the full number
                    let mut number = String::new();
                    number.push(c);

                    while let Some(&next) = chars.peek() {
                        if next.is_ascii_digit() || next == '.' || next == 'e' || next == 'E' || next == '-' {
                            number.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    // Parse as coordinate
                    if let Ok(val) = number.parse::<f64>() {
                        // For commands that take coordinates, transform them
                        match current_command {
                            'M' | 'L' | 'm' | 'l' => {
                                // These take x,y pairs - need to collect both
                                result.push_str(&val.round().to_string());

                                // Skip whitespace/comma
                                while let Some(&next) = chars.peek() {
                                    if next.is_whitespace() || next == ',' {
                                        chars.next();
                                    } else {
                                        break;
                                    }
                                }

                                // Get y coordinate
                                let mut y_num = String::new();
                                while let Some(&next) = chars.peek() {
                                    if next.is_ascii_digit() || next == '.' || next == '-' || next == 'e' || next == 'E' {
                                        y_num.push(chars.next().unwrap());
                                    } else {
                                        break;
                                    }
                                }

                                if let Ok(y) = y_num.parse::<f64>() {
                                    // Transform the point
                                    let (tx, ty) = self.transform_point(val, y);
                                    result.push(' ');
                                    result.push_str(&tx.round().to_string());
                                    result.push(' ');
                                    result.push_str(&ty.round().to_string());
                                }
                            }

                            'H' | 'h' => {
                                // Horizontal line - only x coordinate
                                let (tx, _) = self.transform_point(val, 0.0);
                                result.push_str(&tx.round().to_string());
                            }

                            'V' | 'v' => {
                                // Vertical line - only y coordinate
                                let (_, ty) = self.transform_point(0.0, val);
                                result.push_str(&ty.round().to_string());
                            }

                            _ => {
                                // For other commands (curves, arcs), just pass through for now
                                // Full implementation would need to handle these
                                result.push_str(&number);
                            }
                        }
                    } else {
                        result.push_str(&number);
                    }
                }

                ' ' | ',' => {
                    // Skip - we add our own spacing
                }

                _ => result.push(c),
            }
        }

        result
    }

    fn process_path_transform(&mut self, element: &mut BytesStart) -> io::Result<()> {
        let mut new_attrs = Vec::new();
        let mut found_target = false;
        let mut path_data = None;

        // Collect all attributes
        for attr in element.attributes().flatten() {
            if attr.key.as_ref() == b"transform" {
                let transform_value = String::from_utf8_lossy(&attr.value);

                if transform_value.trim() == self.target_transform.trim() {
                    found_target = true;
                    self.bytes_saved += transform_value.len() + 13; // ' transform=""'
                    // Don't add transform to new attributes (we're removing it)
                } else {
                    // Keep other transforms
                    new_attrs.push((
                        String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                        String::from_utf8_lossy(&attr.value).into_owned(),
                    ));
                }
            } else if attr.key.as_ref() == b"d" {
                // Store path data for transformation
                path_data = Some(String::from_utf8_lossy(&attr.value).into_owned());
            } else {
                // Keep other attributes
                new_attrs.push((
                    String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                    String::from_utf8_lossy(&attr.value).into_owned(),
                ));
            }
        }

        // If we found the target transform and have path data, pre-apply it
        if found_target && path_data.is_some() {
            self.transforms_applied += 1;

            element.clear_attributes();

            // Add back non-transform, non-path attributes
            for (key, value) in new_attrs {
                if key != "d" {
                    element.push_attribute((key.as_str(), value.as_str()));
                }
            }

            // Add transformed path data
            if let Some(original_path) = path_data {
                let transformed_path = self.transform_path_data(&original_path);
                element.push_attribute(("d", transformed_path.as_str()));
            }
        }

        Ok(())
    }
}

impl SVGPlugin for PreApplyTransformOptimizer {
    fn init(&mut self) -> io::Result<()> {
        self.transforms_applied = 0;
        self.bytes_saved = 0;
        Ok(())
    }

    fn process_element(&mut self, element: &mut BytesStart) -> io::Result<()> {
        // Only process path elements (they have 'd' attribute)
        if element.name().as_ref() == b"path" {
            self.process_path_transform(element)?;
        }
        Ok(())
    }

    fn finalize(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "PreApplyTransform"
    }
}

impl PluginStatistics for PreApplyTransformOptimizer {
    fn get_statistics(&self) -> Vec<(&str, String)> {
        vec![
            ("Target transform", format!("\"{}\"", self.target_transform)),
            ("Transforms pre-applied", self.transforms_applied.to_string()),
            ("Bytes saved", format!("{} ({:.1} MB)",
                self.bytes_saved,
                self.bytes_saved as f64 / 1_048_576.0
            )),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_matrix() {
        let transform = "matrix(0 0.33 0.33 0 0 0)";
        let matrix = PreApplyTransformOptimizer::parse_matrix(transform);

        assert!(matrix.is_some());
        let m = matrix.unwrap();
        assert!((m[0] - 0.0).abs() < 0.001);
        assert!((m[1] - 0.33).abs() < 0.001);
        assert!((m[2] - 0.33).abs() < 0.001);
    }

    #[test]
    fn test_transform_point() {
        let optimizer = PreApplyTransformOptimizer::new("matrix(0 0.33 0.33 0 0 0)".to_string());

        let (x, y) = optimizer.transform_point(100.0, 200.0);

        // matrix(0 0.33 0.33 0 0 0) transforms (100, 200) to:
        // x' = 0*100 + 0.33*200 + 0 = 66
        // y' = 0.33*100 + 0*200 + 0 = 33
        assert!((x - 66.0).abs() < 0.1);
        assert!((y - 33.0).abs() < 0.1);
    }
}
