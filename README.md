<h1 align="center">SVGO RS</h1>

<p align="center">Speedy <a href="https://github.com/svg/svgo">SVGO</a> rewrite in Rust 🦀</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#plugins">Plugins</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#license">License</a>
</p>

## Features

- 🚀 **Fast**: Written in Rust for maximum performance
- 🔧 **Memory Efficient**: Processes SVGs in a streaming fashion
- 🛠 **Configurable**: Fine-tune optimizations to your needs
- 🔌 **Plugin System**: Modular design for easy extensibility
- 📊 **Detailed Statistics**: Get insights into optimization results

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/1070rik/svgo-rs.git
cd svgo-rs

# Build with Cargo
cargo build --release

# Optional: Install globally
cargo install --path .
```

### Requirements

- Rust 1.85 or higher
- Cargo

## Usage

Basic usage:

```bash
# Optimize an SVG file
svgo-rs optimize input.svg output.svg --optimize-paths

# Show available plugins
svgo-rs list-plugins

# Get help
svgo-rs --help
```

### Command-line Options

```bash
USAGE:
    svgo-rs [OPTIONS] <COMMAND>

COMMANDS:
    optimize       Optimize SVG files
    list-plugins   List available plugins
    analyze        Show optimization statistics for an SVG file

OPTIONS:
    -b, --buffer-size <SIZE>    Buffer size in KB [default: 8]
    -v, --verbose               Enable verbose output
    -h, --help                  Print help information
    -V, --version               Print version information
```

### Optimization Options

```bash
svgo-rs optimize [OPTIONS] <INPUT> <OUTPUT>

OPTIONS:
    --optimize-paths           Enable path optimization
    --path-decimals <N>       Decimal places for path optimization [default: 2]
    
TODO OPTIONS:
    --dedupe-gradients        Enable gradient deduplication
    --remove-ids              Remove IDs from elements
    --remove-data-attrs       Remove data-* attributes
    --preserve-ids <IDS>      Preserve specified IDs (comma-separated)
```

## Plugins

### Path Optimizer
Optimizes SVG path data by:
- Reducing decimal place precision
- Removing unnecessary spaces
- Optimizing number formatting

```bash
svgo-rs optimize input.svg output.svg --optimize-paths --path-decimals 2
```

## Performance

SVGO RS is designed for high performance and memory efficiency:
- Streaming processing for minimal memory usage
- Efficient buffer management
- Parallel processing capabilities

Example performance metrics:
```
Processing Statistics:
--------------------
Paths processed: 4,721
Characters saved: 127,834
Processing time: 0.187 seconds
Processing speed: 25,245.98 paths/second
Total time: 0.204 seconds
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development

```bash
# Install development tools
rustup component add rustfmt
rustup component add clippy

# Run tests
cargo test

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [SVGO](https://github.com/svg/svgo)
- Built with [Rust](https://www.rust-lang.org/)
- Uses [quick-xml](https://github.com/tafia/quick-xml) for XML processing
- Uses [clap](https://github.com/clap-rs/clap) for CLI argument parsing
