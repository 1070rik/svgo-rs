use crate::plugins::traits::{PluginStatistics, SVGPlugin};
use quick_xml::events::BytesStart;
use std::io;

/// Optimizer that removes a specific common transform from all elements.
///
/// This is useful when you know a specific transform appears thousands of times
/// and you want to either:
/// 1. Remove it entirely (if it's safe to do so)
/// 2. Replace it with a shorter equivalent
///
/// Example use case: SVG exported from CAD software with the same transform on every clipPath
pub struct RemoveCommonTransformOptimizer {
    /// The exact transform string to remove (e.g., "matrix(0,.3333333,.3333333,0,0,0)")
    target_transform: String,

    /// Replacement transform (None = remove entirely)
    replacement: Option<String>,

    /// Count of transforms removed
    removed_count: usize,

    /// Bytes saved
    bytes_saved: usize,
}

impl RemoveCommonTransformOptimizer {
    pub fn new(target_transform: String) -> Self {
        Self {
            target_transform,
            replacement: None,
            removed_count: 0,
            bytes_saved: 0,
        }
    }

    pub fn with_replacement(target_transform: String, replacement: String) -> Self {
        Self {
            target_transform,
            replacement: Some(replacement),
            removed_count: 0,
            bytes_saved: 0,
        }
    }

    fn optimize_element_transform(&mut self, element: &mut BytesStart) -> io::Result<()> {
        let mut new_attrs = Vec::new();
        let mut found_target = false;
        let mut original_size = 0;

        // Collect all attributes
        for attr in element.attributes().flatten() {
            if attr.key.as_ref() == b"transform" {
                let transform_value = String::from_utf8_lossy(&attr.value);

                if transform_value.trim() == self.target_transform.trim() {
                    found_target = true;
                    original_size = transform_value.len() + 13; // ' transform=""'

                    // If we have a replacement, use it; otherwise skip this attribute
                    if let Some(ref replacement) = self.replacement {
                        new_attrs.push((
                            String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                            replacement.clone(),
                        ));
                    }
                } else {
                    // Keep other transforms
                    new_attrs.push((
                        String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                        String::from_utf8_lossy(&attr.value).into_owned(),
                    ));
                }
            } else {
                // Keep non-transform attributes
                new_attrs.push((
                    String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                    String::from_utf8_lossy(&attr.value).into_owned(),
                ));
            }
        }

        // If we found and modified the target transform, rebuild attributes
        if found_target {
            self.removed_count += 1;

            let new_size = if let Some(ref replacement) = self.replacement {
                replacement.len() + 13
            } else {
                0
            };

            self.bytes_saved += original_size.saturating_sub(new_size);

            element.clear_attributes();
            for (key, value) in new_attrs {
                element.push_attribute((key.as_str(), value.as_str()));
            }
        }

        Ok(())
    }
}

impl SVGPlugin for RemoveCommonTransformOptimizer {
    fn init(&mut self) -> io::Result<()> {
        self.removed_count = 0;
        self.bytes_saved = 0;
        Ok(())
    }

    fn process_element(&mut self, element: &mut BytesStart) -> io::Result<()> {
        self.optimize_element_transform(element)?;
        Ok(())
    }

    fn finalize(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "RemoveCommonTransform"
    }
}

impl PluginStatistics for RemoveCommonTransformOptimizer {
    fn get_statistics(&self) -> Vec<(&str, String)> {
        vec![
            ("Target transform", format!("\"{}\"", self.target_transform)),
            ("Transforms removed/replaced", self.removed_count.to_string()),
            ("Bytes saved", format!("{} ({:.1} KB)",
                self.bytes_saved,
                self.bytes_saved as f64 / 1024.0
            )),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_exact_transform() {
        let mut optimizer = RemoveCommonTransformOptimizer::new(
            "matrix(0,.3333333,.3333333,0,0,0)".to_string()
        );
        optimizer.init().unwrap();

        let mut elem = BytesStart::new("path");
        elem.push_attribute(("d", "M10 20L30 40"));
        elem.push_attribute(("transform", "matrix(0,.3333333,.3333333,0,0,0)"));

        optimizer.process_element(&mut elem).unwrap();

        assert_eq!(optimizer.removed_count, 1);

        // Check that transform was removed
        let has_transform = elem.attributes()
            .flatten()
            .any(|attr| attr.key.as_ref() == b"transform");
        assert!(!has_transform);

        // Check that other attributes remain
        let has_d = elem.attributes()
            .flatten()
            .any(|attr| attr.key.as_ref() == b"d");
        assert!(has_d);
    }

    #[test]
    fn test_replace_transform() {
        let mut optimizer = RemoveCommonTransformOptimizer::with_replacement(
            "matrix(0,.3333333,.3333333,0,0,0)".to_string(),
            "scale(0.33)".to_string(),
        );
        optimizer.init().unwrap();

        let mut elem = BytesStart::new("g");
        elem.push_attribute(("transform", "matrix(0,.3333333,.3333333,0,0,0)"));

        optimizer.process_element(&mut elem).unwrap();

        assert_eq!(optimizer.removed_count, 1);

        // Check that transform was replaced
        let transform_value = elem.attributes()
            .flatten()
            .find(|attr| attr.key.as_ref() == b"transform")
            .map(|attr| String::from_utf8_lossy(&attr.value).to_string());

        assert_eq!(transform_value, Some("scale(0.33)".to_string()));
    }

    #[test]
    fn test_preserve_different_transforms() {
        let mut optimizer = RemoveCommonTransformOptimizer::new(
            "matrix(0,.3333333,.3333333,0,0,0)".to_string()
        );
        optimizer.init().unwrap();

        let mut elem = BytesStart::new("rect");
        elem.push_attribute(("transform", "translate(10, 20)"));

        optimizer.process_element(&mut elem).unwrap();

        assert_eq!(optimizer.removed_count, 0);

        // Different transform should be preserved
        let transform_value = elem.attributes()
            .flatten()
            .find(|attr| attr.key.as_ref() == b"transform")
            .map(|attr| String::from_utf8_lossy(&attr.value).to_string());

        assert_eq!(transform_value, Some("translate(10, 20)".to_string()));
    }
}
