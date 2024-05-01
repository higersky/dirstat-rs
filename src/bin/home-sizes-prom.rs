use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use clap::Parser;
use dashmap::{DashMap, DashSet};
use dirstat_rs::{AnalyzeConfig, DiskItem, FileInfo};
use serde::{Deserialize, Serialize};
use tracing::info;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_args();
    if config.valid_days > config.parent_valid_days {
        return Err("Parent valid days (-p) should be larger than valid days (-t) ".into());
    }

    let default_dir = PathBuf::from_str("/home")?;
    let target_dir = config.target_dir.as_ref().unwrap_or(&default_dir);
    let file_info = FileInfo::from_path(target_dir, config.apparent)?;
    let cache = config.cache.as_ref().map(|p| Cache::from_file(p));
    let cache = match cache {
        Some(Ok(cache)) => {
            if cache.target_dir != *target_dir {
                eprintln!(
                    "Warning: target dir mismatched. Expected {}, found {}. Ignored.",
                    target_dir.to_string_lossy(),
                    cache.target_dir.to_string_lossy()
                );
                DashMap::new()
            } else if SystemTime::now() > cache.expire {
                eprintln!("Warning: cache expired.");
                DashMap::new()
            } else if let FileInfo::Directory { volume_id, .. } = file_info {
                if volume_id != cache.volume_id {
                    eprintln!("Warning: volume id mismatched. Ignored.");
                    DashMap::new()
                } else {
                    info!("loading cache");
                    DashMap::from_iter(cache.data)
                }
            } else {
                info!("Loading cache");
                DashMap::from_iter(cache.data)
            }
        }
        Some(Err(e)) => {
            eprintln!("Warning: Failed to load cache - {}", e);
            DashMap::new()
        }
        None => {
            // eprintln!("Cache not found");
            DashMap::new()
        }
    };

    if cache.is_empty() {
        info!("new cache created");
    } else {
        info!("cache loaded. size = {}", cache.len());
    }

    let cache_used = DashSet::new();
    let max_depth = if !cache.is_empty() {
        config.max_depth + 1
    } else {
        2
    };

    let vol_id;
    let analysed = match file_info {
        FileInfo::Directory { volume_id, .. } => {
            vol_id = volume_id;
            if config.cache.is_some() {
                DiskItem::with_cache(
                    target_dir,
                    AnalyzeConfig {
                        root_dev: volume_id,
                        cache_valid_duration: Duration::from_secs(60 * 60 * 24 * config.valid_days),
                        parent_cold_duration: Duration::from_secs(
                            60 * 60 * 24 * config.parent_valid_days,
                        ),
                        apparent: config.apparent,
                        sort: false,
                    },
                    max_depth,
                    &cache,
                    &cache_used,
                )?
            } else {
                DiskItem::from_analyze(target_dir, config.apparent, volume_id, max_depth)?
            }
        }
        _ => return Err(format!("{} is not a directory!", target_dir.display()).into()),
    };

    if !config.show_folder_size {
        show(&analysed);
    } else {
        println!("{}", analysed.disk_size);
    }

    if let Some(cache_path) = config.cache {
        let save = Cache::new(
            vol_id,
            target_dir,
            cache
                .into_iter()
                .filter(|x| cache_used.contains(&x.0))
                .collect(),
            SystemTime::now()
                .checked_add(Duration::from_secs(60 * 60 * config.expire_hours))
                .unwrap_or(SystemTime::UNIX_EPOCH),
        );
        save.save_to_file(&cache_path)?;
    }
    Ok(())
}

fn show(analyzed: &DiskItem) {
    let name = analyzed.name.replace(' ', "_");
    println!("# HELP node_{name}_folder_size_bytes Summarized sizes of subdirectories under folder {name}");
    println!("# TYPE node_{name}_folder_size_bytes gauge");
    for item in analyzed.children.as_ref().expect("BUG: Item has no child") {
        println!(
            "node_{}_folder_size_bytes{{name=\"{}\"}} {}",
            name, item.name, item.disk_size
        );
    }
}

#[derive(Parser)]
struct Config {
    #[clap(short = 'd', default_value = "2")]
    /// Maximum recursion depth in directory for caches.
    max_depth: usize,

    #[clap(short = 'a')]
    /// Apparent size on disk.
    ///
    /// This would actually retrieve allocation size of files (AKA physical size on disk)
    apparent: bool,

    #[clap(short = 's')]
    /// Show the folder's size instead of prom data
    show_folder_size: bool,

    #[clap(short = 'c', parse(from_os_str))]
    /// Cache file path
    cache: Option<PathBuf>,

    #[clap(short = 'e', default_value_t = 24u64)]
    expire_hours: u64,

    /// Duration to consider the cached size of a cold folder reliable
    #[clap(short = 't', default_value_t = 7u64)]
    valid_days: u64,

    /// Duration to consider the cached size of a parent cold folder reliable
    #[clap(short = 'p', default_value_t = 365u64)]
    parent_valid_days: u64,

    #[clap(parse(from_os_str))]
    /// Analyze dir
    target_dir: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    volume_id: u64,
    expire: SystemTime,
    target_dir: PathBuf,
    data: Vec<(u64, (SystemTime, u64))>,
}

impl Cache {
    pub fn from_file(cache_path: &Path) -> Result<Cache, Box<dyn Error>> {
        let f = File::open(cache_path)?;
        let reader = BufReader::new(f);
        let cache: Cache = rmp_serde::from_read(reader)?;
        Ok(cache)
    }

    pub fn new(
        volume_id: u64,
        target_dir: &Path,
        data: Vec<(u64, (SystemTime, u64))>,
        expire: SystemTime,
    ) -> Cache {
        Cache {
            volume_id,
            target_dir: target_dir.to_owned(),
            data,
            expire,
        }
    }

    pub fn save_to_file(&self, cache_path: &Path) -> Result<(), Box<dyn Error>> {
        let f = File::create(cache_path)?;
        let mut writer = BufWriter::new(f);
        let bin = rmp_serde::to_vec(self)?;
        writer.write_all(&bin)?;
        Ok(())
    }
}
