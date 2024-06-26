use clap::Parser;
use dirstat_rs::{DiskItem, FileInfo};
use is_terminal::IsTerminal;
use std::env;
use std::error::Error;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

const INDENT_COLOR: Option<Color> = Some(Color::Rgb(75, 75, 75));

mod shape {
    pub const INDENT: &str = "│";
    pub const _LAST_WITH_CHILDREN: &str = "└─┬";
    pub const LAST: &str = "└──";
    pub const ITEM: &str = "├──";
    pub const _ITEM_WITH_CHILDREN: &str = "├─┬";
    pub const SPACING: &str = "──";
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_args();
    let current_dir = env::current_dir()?;
    let target_dir = config.target_dir.as_ref().unwrap_or(&current_dir);
    let file_info = FileInfo::from_path(target_dir, config.apparent)?;

    let color_choice = if std::io::stdout().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };

    let stdout = BufferWriter::stdout(color_choice);
    let mut buffer = stdout.buffer();

    if !config.json {
        println!("\nAnalyzing: {}\n", target_dir.display())
    };

    let analysed = match file_info {
        FileInfo::Directory { volume_id, .. } => {
            DiskItem::from_analyze(target_dir, config.apparent, volume_id, config.max_depth + 1)?
        }
        _ => return Err(format!("{} is not a directory!", target_dir.display()).into()),
    };

    if config.json {
        let serialized = serde_json::to_string(&analysed)?;
        writeln!(&mut buffer, "{}", serialized)?;
    } else {
        show(&analysed, &config, &DisplayInfo::new(), &mut buffer)?;
    }

    stdout.print(&buffer)?;
    Ok(())
}

fn show(item: &DiskItem, conf: &Config, info: &DisplayInfo, buffer: &mut Buffer) -> io::Result<()> {
    // Show self
    show_item(item, info, buffer)?;
    // Recursively show children
    if info.level < conf.max_depth {
        if let Some(children) = &item.children {
            let children = children
                .iter()
                .map(|child| (child, size_fraction(child, item)))
                .filter(|&(_, fraction)| fraction > conf.min_percent)
                .collect::<Vec<_>>();

            if let Some((last_child, children)) = children.split_last() {
                for &(child, fraction) in children.iter() {
                    show(child, conf, &info.add_item(fraction), buffer)?;
                }
                let &(child, fraction) = last_child;
                show(child, conf, &info.add_last(fraction), buffer)?;
            }
        }
    }
    Ok(())
}

fn show_item(item: &DiskItem, info: &DisplayInfo, buffer: &mut Buffer) -> io::Result<()> {
    // Indentation
    buffer.set_color(ColorSpec::new().set_fg(INDENT_COLOR))?;
    write!(buffer, "{}{}", info.indents, info.prefix())?;
    // Percentage
    buffer.set_color(ColorSpec::new().set_fg(info.color()))?;
    write!(buffer, " {:.2}% ", info.fraction)?;
    // Disk size
    buffer.reset()?;
    write!(
        buffer,
        "[{}]",
        human_bytes::human_bytes(item.disk_size as f64),
    )?;
    // Arrow
    buffer.set_color(ColorSpec::new().set_fg(INDENT_COLOR))?;
    write!(buffer, " {} ", shape::SPACING)?;
    // Name
    buffer.reset()?;
    writeln!(buffer, "{}", item.name)?;
    Ok(())
}

fn size_fraction(child: &DiskItem, parent: &DiskItem) -> f64 {
    100.0 * (child.disk_size as f64 / parent.disk_size as f64)
}

#[derive(Debug, Clone)]
struct DisplayInfo {
    fraction: f64,
    level: usize,
    last: bool,
    indents: String,
}

impl DisplayInfo {
    fn new() -> Self {
        Self {
            fraction: 100.0,
            level: 0,
            last: true,
            indents: String::new(),
        }
    }
    // TODO: Consume or mut instead of cloning
    fn add_item(&self, fraction: f64) -> Self {
        Self {
            fraction,
            level: self.level + 1,
            last: false,
            indents: self.indents.clone() + self.indent() + "  ",
        }
    }

    fn add_last(&self, fraction: f64) -> Self {
        Self {
            fraction,
            level: self.level + 1,
            last: true,
            indents: self.indents.clone() + self.indent() + "  ",
        }
    }

    fn indent(&self) -> &'static str {
        if self.last {
            " "
        } else {
            shape::INDENT
        }
    }

    fn prefix(&self) -> &'static str {
        if self.last {
            shape::LAST
        } else {
            shape::ITEM
        }
    }

    fn color(&self) -> Option<Color> {
        if self.level == 0 {
            Some(Color::Green)
        } else if self.fraction > 20.0 {
            Some(Color::Red)
        } else {
            Some(Color::Cyan)
        }
    }
}

#[derive(Parser)]
struct Config {
    #[clap(short = 'd', default_value = "1")]
    /// Maximum recursion depth in directory.
    max_depth: usize,

    #[clap(
        short = 'm',
        default_value = "0.1",
        parse(try_from_str = parse_percent)
    )]
    /// Threshold that determines if entry is worth
    /// being shown. Between 0-100 % of dir size.
    min_percent: f64,

    #[clap(parse(from_os_str))]
    target_dir: Option<PathBuf>,

    #[clap(short = 'a')]
    /// Apparent size on disk.
    ///
    /// This would actually retrieve allocation size of files (AKA physical size on disk)
    apparent: bool,

    #[clap(short = 'j')]
    /// Output sorted json.
    json: bool,
}

fn parse_percent(src: &str) -> Result<f64, String> {
    let num = src.parse::<f64>().map_err(|x| x.to_string())?;
    if (0.0..=100.0).contains(&num) {
        Ok(num)
    } else {
        Err("Percentage must be in range [0, 100].".into())
    }
}
