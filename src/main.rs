use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use image::{DynamicImage, GenericImageView, GrayImage, Luma, Rgba};
use std::io::{BufWriter, Read, Write};

// ============================================================================
// CLI Configuration
// ============================================================================

#[derive(Clone, Debug, ValueEnum)]
enum Charset {
    Short,
    Long,
    Braille,
    VerticalBlocks,
    VerticalHorizontalBlocks,
    ShadeBlocks,
    Custom,
}

#[derive(Clone, Debug, ValueEnum, PartialEq)]
enum RenderStyle {
    Grayscale,
    Color,
    Background,
    HalfBlock,
    Edge,
}

#[derive(Parser)]
#[command(name = "img2ascii")]
#[command(about = "Convert images to ASCII")]
struct Args {
    /// Path to image file or URL (omit to read from stdin)
    #[arg(help = "Path to image file or URL (omit to read from stdin)")]
    file: Option<String>,

    /// Width of output in characters (or 'fit' for terminal width)
    #[arg(
        long,
        short = 'w',
        default_value = "80",
        help = "Width in characters (or 'fit' for terminal width)"
    )]
    width: String,

    /// Character set
    #[arg(long, short = 'c', value_enum, default_value_t = Charset::Short, help = "Character set to use")]
    charset: Charset,

    /// Custom chars (only read if --charset custom is chosen)
    #[arg(long, default_value = None, help = "Custom character set (required when --charset custom)")]
    custom_chars: Option<String>,

    /// Rendering style
    #[arg(long, short = 's', value_enum, default_value_t = RenderStyle::Grayscale, help = "Rendering style")]
    style: RenderStyle,

    /// Inverting the characters may make it pop more on dark-on-light.
    #[arg(long, help = "Invert the density of the characters")]
    invert: bool,

    /// Character aspect ratio (height/width)
    #[arg(
        long,
        default_value = "2.1",
        help = "Character aspect ratio (height/width)"
    )]
    aspect_ratio: f32,

    /// Exact output height in characters (overrides aspect ratio calculation)
    #[arg(
        long,
        help = "Exact output height in characters (overrides aspect ratio)"
    )]
    height: Option<u32>,

    /// Output file (omit for stdout)
    #[arg(long, short = 'o', help = "Output file (omit for stdout)")]
    output: Option<String>,

    /// Apply Floyd-Steinberg dithering for smoother gradients
    #[arg(long, help = "Apply Floyd-Steinberg dithering")]
    dither: bool,

    /// Rotate image (90, 180, or 270 degrees clockwise)
    #[arg(long, help = "Rotate image (90, 180, or 270 degrees clockwise)")]
    rotate: Option<u16>,

    /// Blur sigma (higher = more blur)
    #[arg(long, help = "Blur sigma (0.0 = none. Reasonable values: 1.0-5.0)")]
    blur: Option<f32>,

    /// Sharpen amount (higher = more sharpen)
    #[arg(long, help = "Sharpen amount (0.0 = none. Reasonable values: 1.0-3.0)")]
    sharpen: Option<f32>,

    /// Flip image horizontally
    #[arg(long, help = "Flip image horizontally")]
    flip_h: bool,

    /// Flip image vertically
    #[arg(long, help = "Flip image vertically")]
    flip_v: bool,

    /// Brightness adjustment (-1.0 to 1.0)
    #[arg(long, default_value = "0.0", help = "Brightness adjustment (-1.0 to 1.0)")]
    brightness: f32,

    /// Contrast adjustment (-1.0 to 1.0)
    #[arg(long, default_value = "0.0", help = "Contrast adjustment (-1.0 to 1.0)")]
    contrast: f32,
}

// ============================================================================
// Image Loading
// ============================================================================

fn load_image(file: &Option<String>) -> Result<DynamicImage> {
    match file {
        Some(path) if path.starts_with("http://") || path.starts_with("https://") => {
            let response = ureq::get(path).call().context("Failed to fetch URL")?;
            let buffer = response
                .into_body()
                .read_to_vec()
                .context("Failed to read response body")?;
            image::load_from_memory(&buffer).context("Failed to decode image from URL")
        }
        Some(path) => {
            if !std::path::Path::new(path).exists() {
                bail!("File '{}' not found", path);
            }
            image::open(path).with_context(|| format!("Failed to open '{}'", path))
        }
        None => {
            let mut buffer = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buffer)
                .context("Failed to read from stdin")?;
            image::load_from_memory(&buffer).context("Failed to decode image from stdin")
        }
    }
}

// ============================================================================
// Character Sets
// ============================================================================

fn build_charset(charset: &Charset, custom: &Option<String>, invert: bool) -> Result<Vec<char>> {
    let mut chars: Vec<char> = match charset {
        Charset::Short => " .:-=+*#%@".chars().collect(),
        Charset::Long => {
            " .'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@"
                .chars()
                .collect()
        }
        Charset::Braille => {
            "⠀⠁⠂⠄⠈⠐⠠⡀⢀⣀⠃⠅⠆⠉⠊⠌⠑⠒⠔⠘⠡⠢⠤⠨⠰⡁⡂⡄⡈⡐⡠⢁⢂⢄⢈⢐⢠⣁⣂⣄⣈⣐⣠⠇⠋⠍⠎⠓⠕⠖⠙⠚⠜⠣⠥⠦⠩⠪⠬⠱⠲⠴⠸⡃⡅⡆⡉⡊⡌⡑⡒⡔⡘⡡⡢⡤⡨⡰⢃⢅⢆⢉⢊⢌⢑⢒⢔⢘⢡⢢⢤⢨⢰⣃⣅⣆⣉⣊⣌⣑⣒⣔⣘⣡⣢⣤⣨⣰⠏⠗⠛⠝⠞⠫⠭⠮⠳⠵⠶⠹⠺⠼⡇⡋⡍⡎⡓⡕⡖⡙⡚⡜⡣⡥⡦⡩⡪⡬⡱⡲⡴⡸⢇⢋⢍⢎⢓⢕⢖⢙⢚⢜⢣⢥⢦⢩⢪⢬⢱⢲⢴⢸⣇⣋⣍⣎⣓⣕⣖⣙⣚⣜⣣⣥⣦⣩⣪⣬⣱⣲⣴⣸⠟⠯⠷⠻⠽⠾⡏⡗⡛⡝⡞⡫⡭⡮⡳⡵⡶⡹⡺⡼⢏⢗⢛⢝⢞⢫⢭⢮⢳⢵⢶⢹⢺⢼⣏⣗⣛⣝⣞⣫⣭⣮⣳⣵⣶⣹⣺⣼⠿⡟⡯⡷⡻⡽⡾⢟⢯⢷⢻⢽⢾⣟⣯⣷⣻⣽⣾⡿⢿⣿"
                .chars()
                .collect()
        }
        Charset::VerticalBlocks => " ▁▂▃▄▅▆▇█".chars().collect(),
        // Interleaved by approximate fill: ▏=1/8h, ▁=1/8v, ▎=1/4h, ▂=1/4v, etc.
        Charset::VerticalHorizontalBlocks => " ▏▁▎▂▍▃▌▄▋▅▊▆▉▇█".chars().collect(),
        Charset::ShadeBlocks => " ░▒▓█".chars().collect(),
        Charset::Custom => {
            let s = custom
                .as_ref()
                .context("Custom chars must be specified when using --charset custom")?;
            s.chars().collect()
        }
    };

    if invert {
        chars.reverse();
    }

    Ok(chars)
}

// ============================================================================
// Dimensions
// ============================================================================

fn calculate_output_dimensions(args: &Args, img: &DynamicImage) -> Result<(u32, u32)> {
    let (img_width, img_height) = img.dimensions();

    let target_width = if args.width == "fit" {
        if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
            (w as u32).saturating_sub(1)
        } else {
            eprintln!("Warning: Could not detect terminal size. Using 80");
            80
        }
    } else {
        args.width
            .parse::<u32>()
            .context("--width must be a number or 'fit'")?
    };

    if let Some(h) = args.height {
        return Ok((target_width, h));
    }

    let new_height =
        (img_height as f32 * target_width as f32) / (img_width as f32 * args.aspect_ratio);
    let new_height = new_height as u32;

    if args.style == RenderStyle::HalfBlock {
        Ok((target_width, new_height * 2))
    } else {
        Ok((target_width, new_height))
    }
}

// ============================================================================
// Image Preprocessing
// ============================================================================

fn preprocess_image(img: DynamicImage, args: &Args) -> Result<DynamicImage> {
    let img = match args.rotate {
        Some(90) => img.rotate90(),
        Some(180) => img.rotate180(),
        Some(270) => img.rotate270(),
        Some(_) => bail!("--rotate must be 90, 180, or 270"),
        None => img,
    };

    let img = if args.flip_h { img.fliph() } else { img };
    let img = if args.flip_v { img.flipv() } else { img };
    let img = if let Some(sigma) = args.blur {
        img.blur(sigma)
    } else {
        img
    };
    let img = if let Some(amount) = args.sharpen {
        img.unsharpen(amount, 2)
    } else {
        img
    };

    // Brightness: map -1.0..1.0 to -255..255 for the image crate's brighten
    let img = if args.brightness != 0.0 {
        img.brighten((args.brightness * 255.0) as i32)
    } else {
        img
    };

    // Contrast: map -1.0..1.0 to -100..100 for the image crate's adjust_contrast
    let img = if args.contrast != 0.0 {
        img.adjust_contrast(args.contrast * 100.0)
    } else {
        img
    };

    Ok(img)
}

// ============================================================================
// Pixel Helpers
// ============================================================================

/// Perceptual brightness (ITU-R BT.601 luma), alpha-blended against white.
fn calculate_brightness(pixel: &Rgba<u8>) -> f32 {
    let alpha = pixel[3] as f32 / 255.0;
    let r = pixel[0] as f32 * alpha + 255.0 * (1.0 - alpha);
    let g = pixel[1] as f32 * alpha + 255.0 * (1.0 - alpha);
    let b = pixel[2] as f32 * alpha + 255.0 * (1.0 - alpha);
    0.299 * r + 0.587 * g + 0.114 * b
}

fn brightness_to_index(brightness: f32, char_count: usize) -> usize {
    ((brightness as usize * char_count) / 256).min(char_count - 1)
}

// ============================================================================
// Edge Detection (Sobel)
// ============================================================================

fn sobel_edge_detect(img: &DynamicImage) -> GrayImage {
    let gray = img.to_luma8();
    let (w, h) = gray.dimensions();
    let mut output = GrayImage::new(w, h);

    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let tl = gray.get_pixel(x - 1, y - 1)[0] as f32;
            let tc = gray.get_pixel(x, y - 1)[0] as f32;
            let tr = gray.get_pixel(x + 1, y - 1)[0] as f32;
            let ml = gray.get_pixel(x - 1, y)[0] as f32;
            let mr = gray.get_pixel(x + 1, y)[0] as f32;
            let bl = gray.get_pixel(x - 1, y + 1)[0] as f32;
            let bc = gray.get_pixel(x, y + 1)[0] as f32;
            let br = gray.get_pixel(x + 1, y + 1)[0] as f32;

            let gx = -tl + tr - 2.0 * ml + 2.0 * mr - bl + br;
            let gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;
            let mag = (gx * gx + gy * gy).sqrt().min(255.0) as u8;

            output.put_pixel(x, y, Luma([mag]));
        }
    }

    output
}

// ============================================================================
// Rendering
// ============================================================================

fn render_half_block(
    writer: &mut impl Write,
    img: &DynamicImage,
) -> Result<()> {
    let (w, h) = img.dimensions();
    for y in (0..h).step_by(2) {
        for x in 0..w {
            let top = img.get_pixel(x, y);
            let bottom = if y + 1 < h {
                img.get_pixel(x, y + 1)
            } else {
                top
            };
            write!(
                writer,
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀\x1b[0m",
                top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]
            )?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn render_edge(
    writer: &mut impl Write,
    img: &DynamicImage,
    chars: &[char],
    invert: bool,
) -> Result<()> {
    let edges = sobel_edge_detect(img);
    let char_count = chars.len();
    let (w, h) = edges.dimensions();

    for y in 0..h {
        for x in 0..w {
            let mut brightness = edges.get_pixel(x, y)[0] as f32;
            // Edge detection: high values = edges. Without invert, we want edges to be
            // the densest characters. Since chars go light-to-dense, we invert by default
            // for edge mode (unless the user already inverted).
            if !invert {
                brightness = 255.0 - brightness;
            }
            let idx = brightness_to_index(brightness, char_count);
            write!(writer, "{}", chars[idx])?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn render_standard(
    writer: &mut impl Write,
    img: &DynamicImage,
    chars: &[char],
    style: &RenderStyle,
    dither: bool,
) -> Result<()> {
    let char_count = chars.len();
    let (w, h) = img.dimensions();

    // For dithering, maintain error buffers using the circular buffer trick:
    // the next row is whatever the opposite parity of the current row is, mod 2.
    let mut error_buffer: Vec<Vec<f32>> = if dither {
        vec![vec![0.0; w as usize]; 2]
    } else {
        vec![]
    };

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);

            let mut brightness = calculate_brightness(&pixel);

            if dither {
                let current_row = (y % 2) as usize;
                brightness += error_buffer[current_row][x as usize];
                brightness = brightness.clamp(0.0, 255.0);
            }

            let idx = brightness_to_index(brightness, char_count);
            let character = chars[idx];

            match style {
                RenderStyle::Grayscale => write!(writer, "{}", character)?,
                RenderStyle::Color => write!(
                    writer,
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    pixel[0], pixel[1], pixel[2], character
                )?,
                RenderStyle::Background => write!(
                    writer,
                    "\x1b[48;2;{};{};{}m \x1b[0m",
                    pixel[0], pixel[1], pixel[2]
                )?,
                RenderStyle::HalfBlock | RenderStyle::Edge => {
                    unreachable!("handled by dedicated render functions")
                }
            }

            if dither {
                let char_brightness = (idx as f32 * 255.0) / (char_count - 1) as f32;
                let error = brightness - char_brightness;

                let current_row = (y % 2) as usize;
                let next_row = ((y + 1) % 2) as usize;
                let xu = x as usize;

                // Floyd-Steinberg error distribution:
                //        X   7/16
                //  3/16 5/16 1/16
                if x + 1 < w {
                    error_buffer[current_row][xu + 1] += error * 7.0 / 16.0;
                }
                if y + 1 < h {
                    if x > 0 {
                        error_buffer[next_row][xu - 1] += error * 3.0 / 16.0;
                    }
                    error_buffer[next_row][xu] += error * 5.0 / 16.0;
                    if x + 1 < w {
                        error_buffer[next_row][xu + 1] += error * 1.0 / 16.0;
                    }
                }
            }
        }

        if dither {
            let current_row = (y % 2) as usize;
            error_buffer[current_row].fill(0.0);
        }

        writeln!(writer)?;
    }

    Ok(())
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    let args = Args::parse();

    let img = load_image(&args.file)?;
    let img = preprocess_image(img, &args)?;
    let chars = build_charset(&args.charset, &args.custom_chars, args.invert)?;

    let (new_width, new_height) = calculate_output_dimensions(&args, &img)?;
    let img = img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);

    let mut writer: BufWriter<Box<dyn Write>> = if let Some(ref output_path) = args.output {
        let file = std::fs::File::create(output_path)
            .with_context(|| format!("Failed to create output file '{}'", output_path))?;
        BufWriter::new(Box::new(file))
    } else {
        BufWriter::new(Box::new(std::io::stdout().lock()))
    };

    match args.style {
        RenderStyle::HalfBlock => render_half_block(&mut writer, &img)?,
        RenderStyle::Edge => render_edge(&mut writer, &img, &chars, args.invert)?,
        _ => render_standard(&mut writer, &img, &chars, &args.style, args.dither)?,
    }

    writer.flush()?;
    Ok(())
}
