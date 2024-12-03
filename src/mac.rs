use std::os::unix::fs::FileTypeExt;
use std::fs;
use std::sync::Arc;
use parking_lot::Mutex;
use std::collections::HashSet;
use rayon::prelude::*;
use std::ffi::CStr;
use std::os::raw::c_void;
use libc::*;
use std::mem;

pub fn fetch_file_system_with_getattrlistbulk_parallel(path: &str) -> Vec<(String, u64, bool)> {
    println!("Processing directory: {}", path);

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut directories_to_process = vec![path.to_string()];
    let mut visited_directories = HashSet::new();

    while !directories_to_process.is_empty() {
        directories_to_process.retain(|dir| visited_directories.insert(dir.clone()));

        let results_clone = Arc::clone(&results);

        let new_directories: Vec<String> = directories_to_process
            .into_par_iter()
            .flat_map(|dir_path| {
                if dir_path.ends_with(".framework") {
                    // println!("Skipping .framework directory: {}", dir_path);
                    return Vec::new();
                }

                let dir_results = fetch_file_system_with_getattrlistbulk(&dir_path);

                let mut subdirectories = Vec::new();
                let mut local_results = Vec::new();

                for (file_name, size, is_file) in dir_results {
                    let entry_path = if dir_path == "/" {
                        format!("/{}", file_name)
                    } else {
                        format!("{}/{}", dir_path, file_name)
                    };

                    if !is_file {
                        subdirectories.push(entry_path.clone());
                    }

                    local_results.push((entry_path, size, is_file));
                }

                // Safely add results using Mutex
                {
                    let mut results_lock = results_clone.lock();
                    results_lock.extend(local_results);
                }

                subdirectories
            })
            .collect();

        directories_to_process = new_directories;

        // Optional: Sleep to reduce system load
        // sleep(Duration::from_millis(1000));
    }

    Arc::try_unwrap(results)
        .expect("Failed to unwrap Arc")
        .into_inner()
}


fn fetch_file_system_with_getattrlistbulk(path: &str) -> Vec<(String, u64, bool)> {
    // println!("PATH: {}", path);
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_fifo() {
            // println!("Skipping pipe: {}", path);
            return Vec::new();
        } else if metadata.file_type().is_socket() {
            // println!("Skipping socket: {}", path);
            return Vec::new();
        } else if metadata.file_type().is_char_device() {
            // println!("Skipping character device: {}", path);
            return Vec::new();
        }
    }
    let mut results = Vec::new();
    unsafe {
        // Open the directory
        let c_path = format!("{}\0", path);
        let dirfd = open(c_path.as_ptr() as *const i8, O_RDONLY);

        if dirfd < 0 {
            // eprintln!("Failed to open directory: {}", path);
            return results;
        }

        // Define the attribute list
        let mut attrlist = attrlist {
            bitmapcount: ATTR_BIT_MAP_COUNT,
            reserved: 0,
            commonattr: ATTR_CMN_NAME | ATTR_CMN_OBJTYPE | ATTR_CMN_RETURNED_ATTRS,
            volattr: 0,
            dirattr: 0,
            fileattr: ATTR_FILE_TOTALSIZE,
            forkattr: 0,
        };

        // Allocate a buffer for attributes
        let buffer_size = 262144;
        let mut buffer: Vec<u8> = vec![0; buffer_size];
        let buffer_ptr = buffer.as_mut_ptr() as *mut c_void;

        let attrlist_ptr = &mut attrlist as *mut attrlist as *mut c_void;
        let result = getattrlistbulk(dirfd, attrlist_ptr, buffer_ptr, buffer_size, 0);

        if result < 0 {
            let errno_value = *libc::__error();
            let err_msg = CStr::from_ptr(strerror(errno_value)).to_string_lossy();
            eprintln!("getattrlistbulk failed: {}, {}", err_msg, path);
        } else {
            let mut offset = 0;
            for _ in 0..result {
                if offset >= buffer_size {
                    eprintln!("Buffer overrun detected");
                    break;
                }

                let entry = buffer_ptr.add(offset) as *const u8;

                let length = *(entry as *const u32) as usize;
                if length == 0 || offset + length > buffer_size {
                    eprintln!("Invalid entry length");
                    break;
                }

                let mut field = entry.add(mem::size_of::<u32>());
                let attribute_set = *(field as *const attribute_set_t);
                // println!("{}", mem::size_of::<attribute_set_t>());
                field = field.add(mem::size_of::<attribute_set_t>());

                let mut file_name = String::new();
                let mut file_size = 0;
                let mut is_file = false;

                if attribute_set.commonattr & ATTR_CMN_NAME != 0 {
                    let name_info = *(field as *const attrreference_t);
                    // println!("{}", mem::size_of::<attrreference_t>());
                    let name_ptr = field.add(name_info.attr_dataoffset as usize) as *const i8;

                    if name_ptr >= (buffer_ptr.add(buffer_size) as *const i8) || name_ptr < entry as *const i8 {
                        eprintln!("Invalid name pointer detected");
                        break;
                    }

                    file_name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
                    field = field.add(mem::size_of::<attrreference_t>());
                }

                if attribute_set.commonattr & ATTR_CMN_OBJTYPE != 0 {
                    let obj_type = *(field as *const u32);
                    // println!("{}, {}", file_name, obj_type);
                    is_file = obj_type == 1 || obj_type == 5;
                    field = field.add(mem::size_of::<u32>());
                }

                if attribute_set.fileattr & ATTR_FILE_TOTALSIZE != 0 {
                    let file_size_ptr = field as *const u32;
                    if file_size_ptr.is_null() || file_size_ptr.align_offset(mem::align_of::<u32>()) != 0 {
                        eprintln!("Unaligned or null pointer for file size");
                        file_size = 0; // Default to 0 or skip this entry
                    } else {
                        file_size = *file_size_ptr;
                    }
                }

                results.push((file_name, file_size as u64, is_file));
                offset += length;
            }
        }

        close(dirfd);
    }

    results
}