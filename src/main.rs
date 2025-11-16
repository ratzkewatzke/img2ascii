use clap::{Parser, ValueEnum};
use image::GenericImageView;
use std::io::{BufWriter, Read, Write};

#[derive(Clone, Debug, ValueEnum)]
enum Charset {
    Short,
    Long,
    Custom,
}

#[derive(Parser)]
#[command(name = "img2ascii")]
#[command(about = "Convert images to ASCII")]
struct Args {
    /// Path to the image file (omit to read from stdin)
    #[arg(help = "Path to the image file (omit to read from stdin)")]
    file: Option<String>,

    /// Width of the ascii image
    #[arg(long, default_value = "80", help = "Width of the output in characters")]
    width: u32,

    /// Character set
    #[arg(long, value_enum, default_value_t = Charset::Short, help = "Character set to use")]
    charset: Charset,

    /// Custom chars (only read if --charset custom is chosen)
    #[arg(long, default_value = None, help = "Custom character set (required when --charset custom)")]
    custom_chars: Option<String>,

    /// Whether or not you want color (we do our best)
    #[arg(long, help = "Enable colored output")]
    color: bool,

    /// Use background colors with block characters for higher fidelity
    #[arg(
        long,
        help = "Use background colors instead of foreground (implies --color)"
    )]
    background: bool,

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

    /// Auto-fit to terminal width
    #[arg(long, help = "Auto-fit to terminal width (overrides --width)")]
    fit: bool,

    /// Output file (omit for stdout)
    #[arg(long, short = 'o', help = "Output file (omit for stdout)")]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut working_chars: Vec<_> = match args.charset {
        Charset::Short => " .:-=+#%@".to_string(),
        Charset::Long => {
            " .'^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$".to_string()
        }
        Charset::Custom => args.custom_chars.unwrap_or_else(|| {
            eprintln!("Custom chars must be specified when using the --charset custom option.");
            std::process::exit(1);
        }),
    }
    .chars()
    .collect();
    let working_char_len = working_chars.len();

    if args.invert {
        working_chars.reverse();
    }

    let img_result = match args.file {
        Some(path) => image::open(path),
        None => {
            let mut buffer = Vec::new();
            match std::io::stdin().read_to_end(&mut buffer) {
                Ok(_) => image::load_from_memory(&buffer),
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    if let Ok(img) = img_result {
        let (width, height) = img.dimensions();

        // Determine output width: use terminal width if --fit, otherwise use --width
        let new_width = if args.fit {
            if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
                (w as u32).saturating_sub(1) // Leave 1 char margin to avoid wrapping
            } else {
                eprintln!("Warning: Could not detect terminal size. Using default width");
                args.width
            }
        } else {
            args.width
        };

        // Compensate for typical character aspect ratio.
        let new_height =
            ((height as f32 * new_width as f32) / (width as f32 * args.aspect_ratio)) as u32;
        let img = img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);

        // Create writer for either file or stdout
        let mut writer: BufWriter<Box<dyn Write>> = if let Some(output_path) = args.output {
            match std::fs::File::create(&output_path) {
                Ok(file) => BufWriter::new(Box::new(file)),
                Err(e) => {
                    eprintln!("Error creating output file '{}': {}", output_path, e);
                    std::process::exit(1);
                }
            }
        } else {
            let stdout = std::io::stdout();
            BufWriter::new(Box::new(stdout.lock()))
        };

        for y in 0..img.height() {
            for x in 0..img.width() {
                let pixel = img.get_pixel(x, y);

                // Use perceptual brightness (ITU-R BT.601 luma) instead of simple average.
                // Human eyes are most sensitive to green (58.7%), then red (29.9%), then blue (11.4%).
                // This matches how professional image editors convert to grayscale.
                let brightness = (0.299 * pixel[0] as f32
                    + 0.587 * pixel[1] as f32
                    + 0.114 * pixel[2] as f32) as u32;
                let idx =
                    ((brightness as usize * working_char_len) / 256).min(working_char_len - 1);

                // For the sickos: in color mode we're going to use the escape sequences for 24-bit color
                // values and then the last escape sequence clears it.
                if args.background {
                    // Background mode: use solid block with background color for higher fidelity
                    write!(
                        writer,
                        "\x1b[48;2;{};{};{}m \x1b[0m",
                        pixel[0], pixel[1], pixel[2]
                    )
                    .unwrap();
                } else if !args.color {
                    write!(writer, "{}", working_chars[idx]).unwrap();
                } else {
                    write!(
                        writer,
                        "\x1b[38;2;{};{};{}m{}\x1b[0m",
                        pixel[0], pixel[1], pixel[2], working_chars[idx]
                    )
                    .unwrap();
                }
            }
            writeln!(writer).unwrap();
        }
        writer.flush().unwrap();
    } else {
        eprintln!("Can't seem to load that image. Check the path and format.");
    }
}
