use crate::plugins::traits::{PluginStatistics, SVGPlugin};
use quick_xml::events::BytesStart;
use std::collections::HashMap;
use std::io;

/// Optimizer that detects and handles common transform attributes across many elements.
///
/// This is especially useful for SVGs where the same transform is applied to hundreds
/// or thousands of elements (e.g., exported from certain CAD tools).
///
/// Strategies:
/// 1. Count transform occurrences
/// 2. For very common transforms, suggest grouping or pre-application
/// 3. Provide statistics on potential savings
pub struct CommonTransformOptimizer {
    /// Map of transform value -> count
    transform_counts: HashMap<String, TransformInfo>,

    /// Minimum occurrences to be considered "common" (default: 10)
    min_occurrences: usize,

    /// Total elements processed
    total_elements: usize,

    /// Elements with transforms
    elements_with_transforms: usize,
}

#[derive(Debug, Clone)]
struct TransformInfo {
    count: usize,
    /// Approximate bytes saved if deduplicated
    bytes_saved_potential: usize,
    /// Elements that have this transform
    element_types: HashMap<Vec<u8>, usize>,
}

impl CommonTransformOptimizer {
    pub fn new(min_occurrences: usize) -> Self {
        Self {
            transform_counts: HashMap::new(),
            min_occurrences,
            total_elements: 0,
            elements_with_transforms: 0,
        }
    }

    fn analyze_transform(&mut self, element: &BytesStart, transform_value: String) {
        let entry = self
            .transform_counts
            .entry(transform_value.clone())
            .or_insert_with(|| TransformInfo {
                count: 0,
                bytes_saved_potential: 0,
                element_types: HashMap::new(),
            });

        entry.count += 1;

        // Calculate potential bytes saved:
        // Each duplicate adds: ' transform="' + value + '"'
        entry.bytes_saved_potential = (entry.count - 1) * (transform_value.len() + 13);

        // Track which element types use this transform
        let element_name = element.name().as_ref().to_vec();
        *entry.element_types.entry(element_name).or_insert(0) += 1;
    }

    fn get_common_transforms(&self) -> Vec<(String, &TransformInfo)> {
        let mut transforms: Vec<_> = self
            .transform_counts
            .iter()
            .filter(|(_, info)| info.count >= self.min_occurrences)
            .map(|(t, info)| (t.clone(), info))
            .collect();

        // Sort by potential bytes saved (descending)
        transforms.sort_by(|a, b| b.1.bytes_saved_potential.cmp(&a.1.bytes_saved_potential));

        transforms
    }
}

impl SVGPlugin for CommonTransformOptimizer {
    fn init(&mut self) -> io::Result<()> {
        self.transform_counts.clear();
        self.total_elements = 0;
        self.elements_with_transforms = 0;
        Ok(())
    }

    fn process_element(&mut self, element: &mut BytesStart) -> io::Result<()> {
        self.total_elements += 1;

        // Check for transform attribute
        for attr in element.attributes().flatten() {
            if attr.key.as_ref() == b"transform" {
                self.elements_with_transforms += 1;
                let transform_value = String::from_utf8_lossy(&attr.value).to_string();
                self.analyze_transform(element, transform_value);
                break;
            }
        }

        Ok(())
    }

    fn finalize(&mut self) -> io::Result<()> {
        // Print analysis report if verbose mode
        let common = self.get_common_transforms();

        if !common.is_empty() {
            println!("\n🔍 Common Transform Analysis:");
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            for (i, (transform, info)) in common.iter().enumerate() {
                if i >= 5 {
                    eprintln!("\n... and {} more common transforms", common.len() - 5);
                    break;
                }

                eprintln!("\n#{} Transform: \"{}\"", i + 1, transform);
                eprintln!("   Occurrences: {}", info.count);
                eprintln!(
                    "   Potential bytes saved: {} bytes ({:.1} KB)",
                    info.bytes_saved_potential,
                    info.bytes_saved_potential as f64 / 1024.0
                );

                eprintln!("   Used in:");
                for (elem_type, count) in &info.element_types {
                    eprintln!(
                        "     - <{}>: {} times",
                        String::from_utf8_lossy(elem_type),
                        count
                    );
                }

                eprintln!("\n   💡 Optimization suggestions:");
                eprintln!("     • Group elements with the same transform");
                eprintln!("     • Pre-apply transform to path coordinates");
                eprintln!("     • Use CSS classes for repeated transforms");
            }

            eprintln!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "CommonTransformAnalyzer"
    }
}

impl PluginStatistics for CommonTransformOptimizer {
    fn get_statistics(&self) -> Vec<(&str, String)> {
        let common = self.get_common_transforms();
        let total_potential_savings: usize = common
            .iter()
            .map(|(_, info)| info.bytes_saved_potential)
            .sum();

        let most_common = common
            .first()
            .map(|(t, info)| {
                format!(
                    "\"{}\" ({} times)",
                    if t.len() > 50 {
                        format!("{}...", &t[..50])
                    } else {
                        t.to_string()
                    },
                    info.count
                )
            })
            .unwrap_or_else(|| "None".to_string());

        vec![
            ("Elements processed", self.total_elements.to_string()),
            (
                "Elements with transforms",
                self.elements_with_transforms.to_string(),
            ),
            ("Unique transforms", self.transform_counts.len().to_string()),
            ("Common transforms (≥10 uses)", common.len().to_string()),
            ("Most common transform", most_common),
            (
                "Total potential savings",
                format!(
                    "{} bytes ({:.1} KB)",
                    total_potential_savings,
                    total_potential_savings as f64 / 1024.0
                ),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::BytesStart;

    #[test]
    fn test_common_transform_detection() {
        let mut optimizer = CommonTransformOptimizer::new(2);
        optimizer.init().unwrap();

        // Create elements with repeated transform
        for _ in 0..10 {
            let mut elem = BytesStart::new("path");
            elem.push_attribute(("transform", "matrix(0,.3333333,.3333333,0,0,0)"));
            optimizer.process_element(&mut elem).unwrap();
        }

        // Different transform
        let mut elem2 = BytesStart::new("circle");
        elem2.push_attribute(("transform", "translate(10, 20)"));
        optimizer.process_element(&mut elem2).unwrap();

        optimizer.finalize().unwrap();

        let stats = optimizer.get_statistics();
        assert_eq!(optimizer.total_elements, 11);
        assert_eq!(optimizer.elements_with_transforms, 11);

        let common = optimizer.get_common_transforms();
        assert_eq!(common.len(), 1); // Only one transform appears ≥2 times
        assert_eq!(common[0].1.count, 10);
    }

    #[test]
    fn test_potential_savings_calculation() {
        let mut optimizer = CommonTransformOptimizer::new(2);
        optimizer.init().unwrap();

        let transform_val = "matrix(0,.3333333,.3333333,0,0,0)";

        for _ in 0..100 {
            let mut elem = BytesStart::new("path");
            elem.push_attribute(("transform", transform_val));
            optimizer.process_element(&mut elem).unwrap();
        }

        optimizer.finalize().unwrap();

        let common = optimizer.get_common_transforms();
        assert_eq!(common.len(), 1);

        // Expected: (100 - 1) * (len("matrix(0,.3333333,.3333333,0,0,0)") + 13)
        // = 99 * (35 + 13) = 99 * 48 = 4752 bytes
        assert_eq!(common[0].1.bytes_saved_potential, 99 * 48);
    }
}
