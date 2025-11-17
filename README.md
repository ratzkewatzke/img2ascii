# img2ascii

A command-line tool for converting images to ASCII art.

## Installation

Requires Rust 1.70 or later.

```bash
cargo install --path .
```

## Usage

Basic usage:

```bash
img2ascii image.jpg
```

The tool reads from stdin if no file is provided:

```bash
cat image.jpg | img2ascii
```

## Options

### Output Dimensions

- `-w, --width <WIDTH>` - Output width in characters (default: 80). Use "fit" to auto-detect terminal width.
- `--height <HEIGHT>` - Exact output height in characters. Overrides aspect ratio calculation.
- `--aspect-ratio <RATIO>` - Character aspect ratio as height/width (default: 2.1).

### Character Sets

- `-c, --charset <CHARSET>` - Character set to use (default: short).
  - `short` - 9 characters: ` .:-=+#%@`
  - `long` - 65 characters for detailed images
  - `braille` - 256 Braille patterns for smooth gradients
  - `vertical-blocks` - 9 vertical block characters
  - `vertical-horizontal-blocks` - 16 combined block characters
  - `shade-blocks` - 5 shade characters: ` ░▒▓█`
  - `custom` - Use with `--custom-chars`
- `--custom-chars <CHARS>` - Custom character set (required when using `--charset custom`).
- `--invert` - Reverse character order for light-on-dark terminals.

### Rendering Styles

- `-s, --style <STYLE>` - Rendering style (default: grayscale).
  - `grayscale` - Monochrome ASCII characters
  - `color` - Colored ASCII characters using ANSI escape codes
  - `background` - Solid blocks with background colors
  - `half-block` - Uses half-block characters with dual colors for 2x vertical resolution

### Image Processing

- `--rotate <DEGREES>` - Rotate image (90, 180, or 270 degrees clockwise).
- `--blur <SIGMA>` - Apply Gaussian blur. Reasonable values: 1.0-5.0.
- `--sharpen <AMOUNT>` - Apply sharpening. Reasonable values: 1.0-3.0.
- `--flip-h` - Flip image horizontally.
- `--flip-v` - Flip image vertically.

### Output

- `-o, --output <FILE>` - Write output to file instead of stdout.
- `--dither` - Apply Floyd-Steinberg dithering for smoother gradients (grayscale only).

## Examples

Convert image with auto-fit width:

```bash
img2ascii photo.jpg -w fit
```

High-quality conversion with Braille characters:

```bash
img2ascii photo.jpg -c braille -w 200
```

Colored output with half-block mode:

```bash
img2ascii photo.jpg -s half-block -w fit
```

Apply image processing before conversion:

```bash
img2ascii photo.jpg --blur 1.5 --sharpen 2.0 --rotate 90
```

Save to file:

```bash
img2ascii photo.jpg -w 120 -o output.txt
```

Dithered grayscale with shade blocks:

```bash
img2ascii photo.jpg -c shade-blocks --dither
```

## Supported Image Formats

The tool supports common image formats including JPEG, PNG, GIF, BMP, TIFF, and WebP.

## License

This project is provided as-is without warranty.
