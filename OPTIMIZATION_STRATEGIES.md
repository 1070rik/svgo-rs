# SVG Optimizer Performance Optimization Strategies

## Current Architecture Analysis

### What We Do Well ✅

The current implementation in `src/processor.rs:39-76` **already uses streaming**:

```rust
loop {
    match reader.read_event_into(&mut xml_buf) {
        Ok(Event::Eof) => break,
        Ok(event) => {
            let processed_event = self.process_event(event)?;
            writer.write_event(processed_event)?;
            xml_buf.clear(); // Reuses buffer!
        }
    }
}
```

**Benefits:**
- ✅ Processes XML events one at a time (not loading entire file into memory)
- ✅ Reuses buffer with `xml_buf.clear()`
- ✅ Writes output as it processes (streaming output)
- ✅ Uses buffered I/O (`BufReader`/`BufWriter`)
- ✅ O(1) memory usage regardless of file size

### What We Could Improve 🚀

Currently in `process_event()` at line 107-119, **every element goes through every plugin**:

```rust
Event::Start(elem) | Event::Empty(elem) => {
    for plugin in &mut self.plugins {
        plugin.process_element(elem)?;  // Every plugin checks every element!
    }
}
```

## Proposed Optimizations

### 1. **Plugin-Aware Element Filtering** (Easy Win 🎯)

**Problem:** PathOptimizer processes `<circle>`, `<text>`, etc. even though it only cares about `<path>`

**Solution:** Let plugins declare which elements they care about

```rust
// Add to SVGPlugin trait
pub trait SVGPlugin: PluginStatistics {
    fn interested_in(&self, element_name: &[u8]) -> bool {
        true  // Default: process all elements
    }

    fn interested_in_attribute(&self, attr_name: &[u8]) -> bool {
        true  // Default: process all attributes
    }

    // ... existing methods
}

// In PathOptimizer
impl SVGPlugin for PathOptimizerPlugin {
    fn interested_in(&self, element_name: &[u8]) -> bool {
        element_name == b"path"
    }
    // ...
}

// In TransformOptimizer
impl SVGPlugin for TransformOptimizerPlugin {
    fn interested_in_attribute(&self, attr_name: &[u8]) -> bool {
        attr_name == b"transform"
    }
    // ...
}

// In processor
fn process_event<'a>(&mut self, mut event: Event<'a>) -> io::Result<Event<'a>> {
    match &mut event {
        Event::Start(elem) | Event::Empty(elem) => {
            for plugin in &mut self.plugins {
                // Skip plugin if it doesn't care about this element
                if plugin.interested_in(elem.name().as_ref()) {
                    plugin.process_element(elem)?;
                }
            }
        }
        _ => {}
    }
    Ok(event)
}
```

**Expected Gain:**
- 50-90% reduction in unnecessary plugin invocations
- Especially beneficial with many plugins
- Negligible overhead for the check

---

### 2. **Lazy Attribute Parsing** (Medium Complexity)

**Problem:** We currently parse ALL attributes even if no plugin needs them

**Current (in PathOptimizer):**
```rust
for attr in element.attributes().flatten() {  // Parses everything
    if attr.key.as_ref() == b"d" {
        // Only use "d" attribute
    }
}
```

**Better:** Only parse attributes that plugins care about

```rust
// Check if element has the attribute we want before parsing all
if element.try_get_attribute(b"d")?.is_some() {
    // Only then parse all attributes
}
```

**Expected Gain:**
- 20-40% reduction in parsing overhead for elements with many attributes
- Greater benefit on complex SVGs with many unused attributes

---

### 3. **Early Exit for Analysis Mode** (Easy Win 🎯)

**Problem:** Analysis processes the entire file even if we just want statistics

**Solution:** Add sampling/early exit for analysis

```rust
pub struct AnalysisConfig {
    pub sample_size: Option<usize>,  // Only process first N elements
    pub sample_percentage: Option<f32>,  // Or process X% of file
}

// In processor
pub fn analyze_file<P: AsRef<Path>>(
    &mut self,
    input_path: P,
    config: AnalysisConfig,
) -> io::Result<AnalysisStats> {
    let mut elements_processed = 0;
    let max_elements = config.sample_size.unwrap_or(usize::MAX);

    loop {
        match reader.read_event_into(&mut xml_buf) {
            Ok(Event::Eof) => break,
            Ok(event) => {
                match event {
                    Event::Start(_) | Event::Empty(_) => {
                        elements_processed += 1;
                        if elements_processed >= max_elements {
                            break;  // Early exit!
                        }
                    }
                    _ => {}
                }
                // Process event...
            }
        }
    }
}
```

**Expected Gain:**
- 90%+ time reduction for analysis on large files
- Trade-off: Less accurate statistics (but often good enough)

---

### 4. **Attribute Existence Pre-Check** (Easy Win 🎯)

**Problem:** Plugins iterate through all attributes to find the one they care about

**Solution:** Add fast path for "does this element have attribute X?"

```rust
// In processor, before calling plugins
fn element_has_attributes(&self, elem: &BytesStart, attrs: &[&[u8]]) -> bool {
    // Quick scan without full parsing
    let raw = elem.attributes_raw();
    attrs.iter().any(|&attr| raw.windows(attr.len()).any(|w| w == attr))
}

// Use it
fn process_event<'a>(&mut self, mut event: Event<'a>) -> io::Result<Event<'a>> {
    match &mut event {
        Event::Start(elem) | Event::Empty(elem) => {
            // Fast pre-check
            let has_transform = element_has_attributes(elem, &[b"transform"]);
            let has_path_d = elem.name().as_ref() == b"path" &&
                            element_has_attributes(elem, &[b"d"]);

            for plugin in &mut self.plugins {
                // Skip expensive processing if element lacks needed attributes
                if self.should_process_plugin(plugin, elem, has_transform, has_path_d) {
                    plugin.process_element(elem)?;
                }
            }
        }
        _ => {}
    }
    Ok(event)
}
```

**Expected Gain:**
- 30-50% reduction in wasted attribute parsing
- Especially effective when elements lack optimizable attributes

---

### 5. **Parallel Processing** (Complex, High Gain for Large Files)

**Problem:** Single-threaded processing doesn't utilize modern multi-core CPUs

**Solution:** Process independent SVG subtrees in parallel

```rust
use rayon::prelude::*;

// For elements that don't have interdependencies
fn process_independent_elements(&mut self, elements: Vec<BytesStart>) -> Vec<BytesStart> {
    elements.par_iter()
        .map(|elem| {
            let mut elem_copy = elem.clone();
            for plugin in &self.plugins {
                plugin.process_element(&mut elem_copy);
            }
            elem_copy
        })
        .collect()
}
```

**Caveats:**
- Complex due to XML nesting and plugin state
- Only works for stateless plugins
- Memory overhead for buffering elements

**Expected Gain:**
- 2-4x speedup on multi-core systems for large files
- Diminishing returns on small files due to overhead

---

### 6. **Skip Non-SVG Content** (Easy Win 🎯)

**Problem:** We process comments, processing instructions, etc. that can't be optimized

**Solution:** Fast-path skip irrelevant events

```rust
fn process_event<'a>(&mut self, event: Event<'a>) -> io::Result<Event<'a>> {
    match event {
        // Skip these without any processing
        Event::Comment(_) | Event::DocType(_) | Event::Decl(_) | Event::PI(_) => {
            return Ok(event);
        }

        Event::Start(mut elem) | Event::Empty(mut elem) => {
            // Only process these
            for plugin in &mut self.plugins {
                plugin.process_element(&mut elem)?;
            }
            Ok(event)
        }

        _ => Ok(event)
    }
}
```

**Expected Gain:**
- 5-15% reduction in processing time
- More benefit on SVGs with many comments/metadata

---

## Implementation Priority

### Phase 1: Quick Wins (Implement First)
1. ✅ **Plugin-Aware Element Filtering** - Easy, high impact
2. ✅ **Skip Non-SVG Content** - Trivial, decent impact
3. ✅ **Attribute Existence Pre-Check** - Easy, good impact

### Phase 2: Medium Effort
4. **Lazy Attribute Parsing** - Moderate complexity, good gains
5. **Early Exit for Analysis** - Easy for analysis mode specifically

### Phase 3: Advanced (If Needed)
6. **Parallel Processing** - Complex, high gains for large files only

---

## Benchmarking Strategy

Before implementing, establish baselines:

```bash
# Create test SVGs of various sizes
cargo build --release

# Benchmark current implementation
hyperfine 'svgo-rs optimize small.svg out.svg --optimize-paths'
hyperfine 'svgo-rs optimize large.svg out.svg --optimize-paths'

# After each optimization, re-benchmark and compare
```

---

## Memory vs. Speed Trade-offs

| Strategy | Memory Impact | Speed Gain | Complexity |
|----------|---------------|------------|------------|
| Plugin Filtering | None | High | Low |
| Lazy Parsing | None | Medium | Medium |
| Early Exit | None | Very High (analysis) | Low |
| Attribute Pre-Check | None | Medium | Low |
| Parallel Processing | High | Very High | High |
| Skip Non-SVG | None | Low | Very Low |

---

## Can We Skip Reading the Entire File?

**Short Answer:** For **optimization**, no. For **analysis**, yes!

### Why We Must Read Everything for Optimization:
- Need to produce complete, valid SVG output
- Can't skip elements - would corrupt the output
- Cross-references (like `<use>` tags) require processing all elements

### Where We CAN Skip Reading:

1. **Analysis Mode:**
   - Sample first 1000 elements
   - Extrapolate statistics
   - 90% time savings

2. **Validation Mode:**
   - Stop on first error
   - Don't need full file for validation

3. **Preview Mode:**
   - Process just visible elements
   - Skip elements outside viewport

4. **Dry-Run Mode:**
   - Calculate potential savings without writing output
   - Can skip actual attribute modification

---

## Recommended Next Steps

1. **Implement Plugin-Aware Filtering** (biggest bang for buck)
2. **Add benchmarks** to measure impact
3. **Implement Skip Non-SVG Content** (trivial, safe)
4. **Add Analysis Sampling Mode** (new feature + performance win)
5. **Profile with real-world SVGs** to find other hotspots

---

## Example: Plugin-Aware Filtering Implementation

See `PLUGIN_FILTERING_IMPLEMENTATION.md` for complete code example.

**Estimated total speedup from Phase 1 optimizations: 2-3x on typical SVG files**
