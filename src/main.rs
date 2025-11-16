use clap::{Parser, ValueEnum};
use image::GenericImageView;
use std::io::{BufWriter, Read, Write};

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

enum RenderMode {
    Grayscale,
    Color,
    Background,
}

fn calculate_brightness(pixel: &image::Rgba<u8>) -> f32 {
    // Use perceptual brightness (ITU-R BT.601 luma) instead of simple average.
    // Human eyes are most sensitive to green (58.7%), then red (29.9%), then blue (11.4%).
    0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32
}

fn brightness_to_index(brightness: f32, char_count: usize) -> usize {
    ((brightness as usize * char_count) / 256).min(char_count - 1)
}

fn calculate_output_dimensions(
    img_width: u32,
    img_height: u32,
    target_width: u32,
    aspect_ratio: f32,
) -> (u32, u32) {
    let new_height = (img_height as f32 * target_width as f32) / (img_width as f32 * aspect_ratio);
    (target_width, new_height as u32)
}

fn render_pixel(
    writer: &mut impl Write,
    pixel: &image::Rgba<u8>,
    character: char,
    mode: &RenderMode,
) -> std::io::Result<()> {
    match mode {
        RenderMode::Grayscale => write!(writer, "{}", character),
        RenderMode::Color => write!(
            writer,
            "\x1b[38;2;{};{};{}m{}\x1b[0m",
            pixel[0], pixel[1], pixel[2], character
        ),
        RenderMode::Background => write!(
            writer,
            "\x1b[48;2;{};{};{}m \x1b[0m",
            pixel[0], pixel[1], pixel[2]
        ),
    }
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

    /// Exact output height in characters (overrides aspect ratio calculation)
    #[arg(long, help = "Exact output height in characters (overrides aspect ratio)")]
    height: Option<u32>,

    /// Output file (omit for stdout)
    #[arg(long, short = 'o', help = "Output file (omit for stdout)")]
    output: Option<String>,

    /// Apply Floyd-Steinberg dithering for smoother gradients
    #[arg(long, help = "Apply Floyd-Steinberg dithering (grayscale only)")]
    dither: bool,
}

fn main() {
    let args = Args::parse();

    let mut working_chars: Vec<_> = match args.charset {
        Charset::Short => " .:-=+#%@".to_string(),
        Charset::Long => {
                " .'^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$".to_string()
            }
        Charset::Braille => {
                "⠀⠁⠂⠄⠈⠐⠠⡀⢀⣀⠃⠅⠆⠉⠊⠌⠑⠒⠔⠘⠡⠢⠤⠨⠰⡁⡂⡄⡈⡐⡠⢁⢂⢄⢈⢐⢠⣁⣂⣄⣈⣐⣠⠇⠋⠍⠎⠓⠕⠖⠙⠚⠜⠣⠥⠦⠩⠪⠬⠱⠲⠴⠸⡃⡅⡆⡉⡊⡌⡑⡒⡔⡘⡡⡢⡤⡨⡰⢃⢅⢆⢉⢊⢌⢑⢒⢔⢘⢡⢢⢤⢨⢰⣃⣅⣆⣉⣊⣌⣑⣒⣔⣘⣡⣢⣤⣨⣰⠏⠗⠛⠝⠞⠫⠭⠮⠳⠵⠶⠹⠺⠼⡇⡋⡍⡎⡓⡕⡖⡙⡚⡜⡣⡥⡦⡩⡪⡬⡱⡲⡴⡸⢇⢋⢍⢎⢓⢕⢖⢙⢚⢜⢣⢥⢦⢩⢪⢬⢱⢲⢴⢸⣇⣋⣍⣎⣓⣕⣖⣙⣚⣜⣣⣥⣦⣩⣪⣬⣱⣲⣴⣸⠟⠯⠷⠻⠽⠾⡏⡗⡛⡝⡞⡫⡭⡮⡳⡵⡶⡹⡺⡼⢏⢗⢛⢝⢞⢫⢭⢮⢳⢵⢶⢹⢺⢼⣏⣗⣛⣝⣞⣫⣭⣮⣳⣵⣶⣹⣺⣼⠿⡟⡯⡷⡻⡽⡾⢟⢯⢷⢻⢽⢾⣟⣯⣷⣻⣽⣾⡿⢿⣿".to_string()
            }
        Charset::VerticalBlocks => " ▁▂▃▄▅▆▇█".to_string(),
        Charset::VerticalHorizontalBlocks => " ▁▂▃▄▅▆▇▏▎▍▌▋▊▉█".to_string(),
        Charset::ShadeBlocks => " ░▒▓█".to_string(),
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
        let target_width = if args.fit {
            if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
                (w as u32).saturating_sub(1) // Leave 1 char margin to avoid wrapping
            } else {
                eprintln!("Warning: Could not detect terminal size. Using default width");
                args.width
            }
        } else {
            args.width
        };

        let (new_width, new_height) = if let Some(h) = args.height {
            // User specified exact height, ignore aspect ratio
            (target_width, h)
        } else {
            // Calculate height based on aspect ratio (current behavior)
            calculate_output_dimensions(width, height, target_width, args.aspect_ratio)
        };
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

        // For dithering, we need to maintain error buffers for current and next row. This is
        // the old circular buffer trick where your next line is whatever the opposite parity
        // of yours is, mod 2.
        let mut error_buffer: Vec<Vec<f32>> = if args.dither && !args.background {
            vec![vec![0.0; img.width() as usize]; 2]
        } else {
            vec![]
        };

        // Determine render mode
        let render_mode = if args.background {
            RenderMode::Background
        } else if args.color {
            RenderMode::Color
        } else {
            RenderMode::Grayscale
        };

        for y in 0..img.height() {
            for x in 0..img.width() {
                let pixel = img.get_pixel(x, y);

                let mut brightness = calculate_brightness(&pixel);

                // Apply dithering error if enabled.
                if args.dither && !args.background {
                    let current_row = (y % 2) as usize;
                    brightness += error_buffer[current_row][x as usize];
                    brightness = brightness.clamp(0.0, 255.0);
                }

                let idx = brightness_to_index(brightness, working_char_len);
                let character = working_chars[idx];

                render_pixel(&mut writer, &pixel, character, &render_mode).unwrap();

                // If we're dithering, we need to push the error to the other squares.
                if args.dither && !args.background {
                    // Calculate the brightness that the chosen character represents
                    let char_brightness = (idx as f32 * 255.0) / (working_char_len - 1) as f32;
                    let error = brightness - char_brightness;

                    let current_row = (y % 2) as usize;
                    let next_row = ((y + 1) % 2) as usize;
                    let x_usize = x as usize;

                    // Distribute error to neighboring pixels. I stole this from Wikipedia:
                    // https://en.wikipedia.org/wiki/Floyd%E2%80%93Steinberg_dithering
                    //        X   7/16
                    //  3/16 5/16 1/16
                    if x + 1 < img.width() {
                        error_buffer[current_row][x_usize + 1] += error * 7.0 / 16.0;
                    }
                    if y + 1 < img.height() {
                        if x > 0 {
                            error_buffer[next_row][x_usize - 1] += error * 3.0 / 16.0;
                        }
                        error_buffer[next_row][x_usize] += error * 5.0 / 16.0;
                        if x + 1 < img.width() {
                            error_buffer[next_row][x_usize + 1] += error * 1.0 / 16.0;
                        }
                    }
                }
            }

            // Clear current row's error buffer for next iteration
            if args.dither && !args.background {
                let current_row = (y % 2) as usize;
                error_buffer[current_row].fill(0.0);
            }

            writeln!(writer).unwrap();
        }
        writer.flush().unwrap();
    } else {
        eprintln!("Can't seem to load that image. Check the path and format.");
    }
}
