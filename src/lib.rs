use dashmap::DashMap;
use rayon::prelude::*;
use serde::Serialize;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

mod ffi;

#[derive(Serialize)]
pub struct DiskItem {
    pub name: String,
    pub disk_size: u64,
    pub children: Option<Vec<DiskItem>>,
}

impl DiskItem {
    pub fn from_analyze(
        path: &Path,
        apparent: bool,
        root_dev: u64,
        depth_limit: usize,
    ) -> Result<Self, Box<dyn Error>> {
        #[cfg(windows)]
        {
            // Solution for windows compressed files requires path to be absolute, see ffi.rs
            // Basically it would be triggered only on top most invocation,
            // and afterwards all path would be absolute. We do it here as it is relatively harmless
            // but this would allow us fo it only once instead of each invocation of ffi::compressed_size
            if apparent && !path.is_absolute() {
                use path_absolutize::*;
                let absolute_dir = path.absolutize()?;
                return Self::analyze(
                    absolute_dir.as_ref(),
                    apparent,
                    root_dev,
                    &DashMap::new(),
                    depth_limit,
                );
            }
        }
        Self::analyze(path, apparent, root_dev, &DashMap::new(), depth_limit)
    }

    fn analyze(
        path: &Path,
        apparent: bool,
        root_dev: u64,
        fileid_map: &DashMap<u64, u64>,
        depth_limit: usize,
    ) -> Result<Self, Box<dyn Error>> {
        let name = path
            .file_name()
            .unwrap_or(OsStr::new("."))
            .to_string_lossy()
            .to_string();

        let file_info = FileInfo::from_path(path, apparent)?;

        match file_info {
            FileInfo::Directory { volume_id, .. } => {
                if volume_id != root_dev {
                    return Err("Filesystem boundary crossed".into());
                }

                let sub_entries = fs::read_dir(path)?
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>();

                let (mut sub_items, disk_size) = if depth_limit > 0 {
                    let my_fileid_map = DashMap::new();
                    let sub_items = sub_entries
                        .par_iter()
                        .filter_map(|entry| {
                            DiskItem::analyze(
                                &entry.path(),
                                apparent,
                                root_dev,
                                &my_fileid_map,
                                depth_limit - 1,
                            )
                            .ok()
                        })
                        .collect::<Vec<_>>();
                    let disk_size: u64 = sub_items.iter().map(|di| di.disk_size).sum();
                    let repeated_size: u64 = my_fileid_map
                        .into_iter()
                        .map(|(k, v)| {
                            fileid_map.entry(k).and_modify(|x| *x += v).or_insert(0);
                            v
                        })
                        .sum();
                    (sub_items, disk_size - repeated_size)
                } else {
                    let sub_items = sub_entries
                        .par_iter()
                        .filter_map(|entry| {
                            DiskItem::analyze(&entry.path(), apparent, root_dev, fileid_map, 0).ok()
                        })
                        .collect::<Vec<_>>();
                    let disk_size = sub_items.iter().map(|di| di.disk_size).sum();
                    (sub_items, disk_size)
                };

                sub_items.sort_unstable_by(|a, b| a.disk_size.cmp(&b.disk_size).reverse());

                Ok(DiskItem {
                    name,
                    disk_size,
                    children: if depth_limit > 0 {
                        Some(sub_items)
                    } else {
                        None
                    },
                })
            }
            FileInfo::File {
                size,
                file_id: inode,
                ..
            } => {
                fileid_map
                    .entry(inode)
                    .and_modify(|x| *x += size)
                    .or_insert(0);
                Ok(DiskItem {
                    name,
                    disk_size: size,
                    children: None,
                })
            }
        }
    }
}

pub enum FileInfo {
    File {
        size: u64,
        volume_id: u64,
        file_id: u64,
    },
    Directory {
        volume_id: u64,
        file_id: u64,
    },
}

impl FileInfo {
    #[cfg(unix)]
    pub fn from_path(path: &Path, apparent: bool) -> Result<Self, Box<dyn Error>> {
        use std::os::unix::fs::MetadataExt;

        let md = path.symlink_metadata()?;
        if md.is_dir() {
            Ok(FileInfo::Directory {
                volume_id: md.dev(),
                file_id: md.ino(),
            })
        } else {
            let size = if apparent {
                md.blocks() * 512
            } else {
                md.len()
            };
            Ok(FileInfo::File {
                size,
                volume_id: md.dev(),
                file_id: md.ino(),
            })
        }
    }

    #[cfg(windows)]
    pub fn from_path(path: &Path, apparent: bool) -> Result<Self, Box<dyn Error>> {
        use winapi_util::{file, Handle};
        const FILE_ATTRIBUTE_DIRECTORY: u64 = 0x10;

        let h = Handle::from_path_any(path)?;
        let md = file::information(h)?;

        if md.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0 {
            Ok(FileInfo::Directory {
                volume_id: md.volume_serial_number(),
                file_id: md.file_index(),
            })
        } else {
            let size = if apparent {
                ffi::compressed_size(path)?
            } else {
                md.file_size()
            };
            Ok(FileInfo::File {
                size,
                volume_id: md.volume_serial_number(),
                file_id: md.file_index(),
            })
        }
    }
}

#[cfg(test)]
mod tests;
