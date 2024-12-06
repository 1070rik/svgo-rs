use crate::plugins::traits::SVGPlugin;
use quick_xml::events::BytesStart;
use std::io;

pub struct PathOptimizerPlugin {
    decimal_places: usize,
    path_count: usize,
    total_chars_saved: usize,
}

impl PathOptimizerPlugin {
    pub fn new(decimal_places: usize) -> Self {
        Self {
            decimal_places,
            path_count: 0,
            total_chars_saved: 0,
        }
    }

    fn optimize_path_data(&mut self, path_data: &str) -> String {
        let mut optimized = String::with_capacity(path_data.len());
        let mut chars = path_data.chars().peekable();
        let mut prev_was_number = false;

        while let Some(c) = chars.next() {
            match c {
                // SVG path commands
                'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' |
                'C' | 'c' | 'S' | 's' | 'Q' | 'q' | 'T' | 't' |
                'A' | 'a' | 'Z' | 'z' => {
                    prev_was_number = false;
                    optimized.push(c);
                },

                // Numbers, including decimals and negative signs
                '0'..='9' | '.' | '-' => {
                    // Add space between numbers if needed
                    if prev_was_number {
                        optimized.push(' ');
                    }

                    let mut number = String::new();
                    number.push(c);

                    // Collect the full number
                    while let Some(&next) = chars.peek() {
                        if next.is_ascii_digit() || next == '.' || next == 'e' || next == 'E' || next == '-' {
                            number.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    // Parse and round the number
                    if let Ok(num) = number.parse::<f64>() {
                        let rounded = format!("{:.1$}", num, self.decimal_places);
                        // Remove trailing zeros and unnecessary decimal points
                        let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
                        optimized.push_str(trimmed);
                        prev_was_number = true;
                    } else {
                        optimized.push_str(&number);
                        prev_was_number = false;
                    }
                },

                // Whitespace and commas
                ' ' | ',' => {
                    // Only add space if between numbers and not before commands
                    if prev_was_number {
                        if let Some(&next) = chars.peek() {
                            if next.is_ascii_digit() || next == '-' {
                                optimized.push(' ');
                            }
                        }
                    }
                },

                // Preserve any other characters (shouldn't occur in valid path data)
                _ => optimized.push(c),
            }
        }

        self.total_chars_saved += path_data.len() - optimized.len();
        optimized
    }

    pub fn get_statistics(&self) -> (usize, usize) {
        (self.path_count, self.total_chars_saved)
    }
}

impl SVGPlugin for PathOptimizerPlugin {
    fn init(&mut self) -> io::Result<()> {
        self.path_count = 0;
        self.total_chars_saved = 0;
        Ok(())
    }

    // In src/plugins/path.rs, update the process_element method:
    fn process_element(&mut self, element: &mut BytesStart) -> io::Result<()> {
        if element.name().as_ref() == b"path" {
            self.path_count += 1;

            // Convert attributes into fully owned strings
            let mut new_attrs = Vec::new();
            let mut path_data = None;

            // Collect and convert all attributes to owned strings
            for attr in element.attributes().flatten() {
                if attr.key.as_ref() == b"d" {
                    let data = String::from_utf8_lossy(&attr.value).into_owned();
                    path_data = Some(self.optimize_path_data(&data));
                } else {
                    // Store fully owned strings
                    new_attrs.push((
                        String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                        String::from_utf8_lossy(&attr.value).into_owned()
                    ));
                }
            }

            // Now we have fully owned data, we can modify the element
            if let Some(optimized_path) = path_data {
                element.clear_attributes();

                // Add back non-path attributes using owned strings
                for (key, value) in new_attrs {
                    element.push_attribute((key.as_str(), value.as_str()));
                }

                // Add the optimized path
                element.push_attribute(("d", optimized_path.as_str()));
            }
        }
        Ok(())
    }

    fn finalize(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "PathOptimizer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_optimization() {
        let mut optimizer = PathOptimizerPlugin::new(2);

        // Test basic number formatting
        assert_eq!(optimizer.optimize_path_data("M 100.000 200.000"), "M100 200");

        // Test multiple commands
        assert_eq!(
            optimizer.optimize_path_data("M 10.123,20.456 L 30.789,40.012"),
            "M10.12 20.46L30.79 40.01"
        );

        // Test relative commands
        assert_eq!(
            optimizer.optimize_path_data("m 5.123,5.456 l 10.789,10.012"),
            "m5.12 5.46l10.79 10.01"
        );
    }
}
