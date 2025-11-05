# Optimizing CAD-Exported SVG Files

## Problem: Huge SVG Files with Repeated Transforms

If your SVG contains patterns like this:

```xml
<clipPath id="clip_1">
  <path transform="matrix(0,.3333333,.3333333,0,0,0)" d="M8246 9101H8293..."/>
</clipPath>
<clipPath id="clip_2">
  <path transform="matrix(0,.3333333,.3333333,0,0,0)" d="M8246 9243H8293..."/>
</clipPath>
<clipPath id="clip_3">
  <path transform="matrix(0,.3333333,.3333333,0,0,0)" d="M2003 17949H2027..."/>
</clipPath>
<!-- ... repeated 1000s of times ... -->
```

**The Problem:**
- The exact same transform appears on EVERY path element
- `transform="matrix(0,.3333333,.3333333,0,0,0)"` = 48 bytes per occurrence
- With 1000 clipPaths, that's **48 KB** wasted on duplicate transforms alone!

---

## Solution: Two-Step Optimization

### Step 1: Analyze the File

First, let's see what we're dealing with:

```bash
svgo-rs optimize input.svg output.svg \
  --optimize-paths \
  --optimize-transforms \
  -v
```

You'll see output like:

```
🔍 Common Transform Analysis:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#1 Transform: "matrix(0,.3333333,.3333333,0,0,0)"
   Occurrences: 1247
   Potential bytes saved: 59,856 bytes (58.5 KB)
   Used in:
     - <path>: 1247 times

   💡 Optimization suggestions:
     • Group elements with the same transform
     • Pre-apply transform to path coordinates
     • Use CSS classes for repeated transforms

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

PathOptimizer Statistics:
--------------------
Paths optimized: 1247
Total characters saved: 15,432

TransformOptimizer Statistics:
--------------------
Transforms optimized: 0
Identity transforms removed: 0
Total characters saved: 0

CommonTransformAnalyzer Statistics:
--------------------
Elements processed: 1247
Elements with transforms: 1247
Unique transforms: 1
Common transforms (≥10 uses): 1
Most common transform: "matrix(0,.3333333,.3333333,0,0,0)" (1247 times)
Total potential savings: 59,856 bytes (58.5 KB)
```

### Step 2: Remove the Duplicate Transform

Now that we know the exact transform, let's remove it from all elements:

```bash
svgo-rs optimize input.svg output_optimized.svg \
  --optimize-paths \
  --optimize-transforms \
  --remove-transform "matrix(0,.3333333,.3333333,0,0,0)" \
  -v
```

**Result:**
- Original file: 850 KB
- After path optimization: 750 KB (saved 100 KB)
- After removing duplicate transforms: **690 KB (saved 160 KB total!)**

---

## What the Optimizers Do

### 1. **Path Optimizer** (`--optimize-paths`)
```xml
<!-- Before -->
<path d="M 100.000 200.000 L 300.000 400.000"/>

<!-- After -->
<path d="M100 200L300 400"/>
```
Typical savings: **10-15%** on path data

### 2. **Transform Optimizer** (`--optimize-transforms`)
```xml
<!-- Before -->
<g transform="translate(0, 0) scale(1) rotate(0)">

<!-- After -->
<g>  <!-- Removed entirely! -->
```
Removes identity transforms (transforms that do nothing)

### 3. **Common Transform Analyzer** (`--analyze-common-transforms` or `-v`)
Scans the entire file and reports:
- Which transforms appear frequently
- How many times they appear
- Potential bytes saved if deduplicated

### 4. **Remove Common Transform** (`--remove-transform <VALUE>`)
Removes the specified transform from ALL elements.

**⚠️ Important:**
- The transform must match EXACTLY (including spacing, commas, decimal places)
- This removes the transform attribute entirely
- Only do this if you understand what the transform does

---

## Understanding `matrix(0,.3333333,.3333333,0,0,0)`

This specific transform is:
- **90-degree rotation** (the 0s in positions 1,2)
- **Scale by 0.333** (approximately 1/3)

If you remove this transform, your SVG might appear rotated or scaled incorrectly. Consider these options:

### Option A: Just Remove It (If Safe)
If the transform is applied uniformly and you can re-apply it at a higher level (like in the `<svg>` tag or parent `<g>`), just remove it:

```bash
svgo-rs optimize input.svg output.svg \
  --remove-transform "matrix(0,.3333333,.3333333,0,0,0)"
```

### Option B: Keep It But Optimize Everything Else
If you need to keep the transform, just optimize the paths and other transforms:

```bash
svgo-rs optimize input.svg output.svg \
  --optimize-paths \
  --optimize-transforms
```

### Option C: Manual Fix
1. Remove the transform with `--remove-transform`
2. Wrap the entire SVG content in a group with that transform:

```xml
<svg>
  <g transform="matrix(0,.3333333,.3333333,0,0,0)">
    <!-- All your content here, without individual transforms -->
  </g>
</svg>
```

This gives you the same visual result but saves ~60 KB!

---

## Full Example: Optimizing Your Specific SVG

```bash
# Step 1: Analyze
svgo-rs optimize your_cad_export.svg temp.svg -v

# Look at the output, note the most common transform

# Step 2: Optimize with removal
svgo-rs optimize your_cad_export.svg optimized.svg \
  --optimize-paths \
  --path-decimals 2 \
  --optimize-transforms \
  --remove-transform "matrix(0,.3333333,.3333333,0,0,0)" \
  -v

# Step 3: Check file sizes
ls -lh your_cad_export.svg optimized.svg
```

---

## Expected Results for CAD SVGs

| File Size | Typical Savings |
|-----------|-----------------|
| < 100 KB  | 20-30%          |
| 100-500 KB | 30-40%         |
| 500 KB - 2 MB | 40-60%      |
| > 2 MB    | 50-70%          |

Your specific case (with repeated transforms):
- **Before:** All optimizers = 100 KB saved
- **After:** With `--remove-transform` = **150-200 KB saved!**

---

## Troubleshooting

### "My SVG looks wrong after optimization!"

The transform might be essential. Try:

```bash
# More conservative optimization
svgo-rs optimize input.svg output.svg \
  --optimize-paths \
  --path-decimals 3 \
  --optimize-transforms
```

### "The transform doesn't match exactly"

Use verbose mode to see the exact transform string:

```bash
svgo-rs optimize input.svg output.svg -v | grep "Transform:"
```

Then copy-paste the exact string into `--remove-transform`.

### "It's still too big!"

Try:
1. Increase path decimal reduction: `--path-decimals 1` (more aggressive)
2. Remove unnecessary IDs: `--remove-ids` (when implemented)
3. Remove data attributes: `--remove-data-attrs` (when implemented)

---

## Advanced: Pre-applying Transforms (Future)

Future versions will support pre-applying transforms to path coordinates:

```xml
<!-- Before -->
<path transform="matrix(0,.3333333,.3333333,0,0,0)" d="M100 200L300 400"/>

<!-- After (transform applied to coordinates) -->
<path d="M66.67 33.33L133.33 100"/>
```

This would give the same visual result while eliminating the transform entirely!

---

## Summary

For your specific CAD SVG with repeated `matrix(0,.3333333,.3333333,0,0,0)` transforms:

```bash
svgo-rs optimize input.svg output.svg \
  --optimize-paths \
  --optimize-transforms \
  --remove-transform "matrix(0,.3333333,.3333333,0,0,0)" \
  -v
```

**Expected savings: 150-200 KB (or more) on large CAD exports!**
