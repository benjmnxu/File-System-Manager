use std::os::unix::fs::FileTypeExt;
// use std::collections::{HashSet, VecDeque};
// use std::fs::{self, Metadata};
// use std::path::{Path, PathBuf};
// use std::rc::Rc;
// use std::cell::RefCell;
use std::sync::{Arc};
use std::time::Duration;
use parking_lot::{Mutex};
use std::thread::sleep;


// // use rayon::prelude::*;
// use std::os::unix::fs::MetadataExt;
// use tokio::sync::mpsc;
// use std::thread;

// use rust::system::*;
// use rust::kernel::Kernel;
// use rust::gui;
// use rust::threads::*;

// #[tokio::main]
// async fn main() {
//     let (to_backend, mut from_gui) = mpsc::channel(32);
//     let (to_gui, from_backend) = mpsc::channel(32);

//     let backend_handle = tokio::spawn(async move {
//         run_backend(&mut from_gui, &to_gui).await;
//     });

//     // Start the GUI application in the main thread
//     gui::run_app(to_backend, from_backend).unwrap();

//     // Wait for the backend task to finish
//     backend_handle.await.unwrap();
// }


// enum BackendState {
//     Uninitialized,
//     Initialized {
//         fs_root: Arc<Mutex<FileSystemNode>>,
//         kernel: Arc<Mutex<Kernel>>,
//         current_node: Arc<Mutex<FileSystemNode>>,
//     },
// }

// async fn run_backend(from_gui: &mut mpsc::Receiver<Command>, to_gui: &mpsc::Sender<BackendResponse>) {
//     println!("RUNNING BACKEND");
//     let visited_inodes: Arc<Mutex<HashSet<(u64, u64)>>> = Arc::new(Mutex::new(HashSet::new()));
//     let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
//     let mut state = BackendState::Uninitialized;

//     loop {
//         println!("top");
//         match from_gui.recv().await {
//             Some(command) => { 
//                 println!("ASDF");
//                 match &mut state {
//                 BackendState::Uninitialized => match command {
//                     Command::LoadDirectory(path) => {
//                         println!("here");
//                         let mut input =  Vec::new();
//                         input.push(path);
//                         match build_fs_model(
//                             input.clone()
//                         ).await {
//                             Some(fs_root) => {
//                                 let root = fs_root;
//                                 let kernel = Arc::new(Mutex::new(Kernel::new(root.clone())));
//                                 state = BackendState::Initialized {
//                                     fs_root: root.clone(),
//                                     kernel,
//                                     current_node: root,
//                                 };
//                                 send_response(&to_gui, format!("Directory loaded: {}", input[0])).await;
//                             }
//                             None => send_error(&to_gui, "Failed to load directory.".to_string()).await,
//                         }
//                     }
//                     Command::Exit => {
//                         println!("here2");
//                         send_response(&to_gui, "Exiting backend.".to_string());
//                         break;
//                     }
//                     _ => {
//                         println!("here3");
//                         send_error(&to_gui, "Load a directory before issuing commands.".to_string());
//                     }
//                 },
//                 BackendState::Initialized {
//                     fs_root,
//                     kernel,
//                     current_node,
//                 } => {
//                     handle_command(command, fs_root.clone(), kernel.clone(), current_node, &to_gui);
//                 }
//             }},
//             None => break, // Handle sender disconnect
//         }
//     }
// }

// fn handle_command(
//     command: Command,
//     fs_root: Arc<Mutex<FileSystemNode>>,
//     kernel: Arc<Mutex<Kernel>>,
//     current_node: &mut Arc<Mutex<FileSystemNode>>,
//     to_gui: &mpsc::Sender<BackendResponse>,
// ) {
//     match command {
//         Command::Del(index) => {
//             kernel.lock().unwrap().mark_for_deletion(current_node.clone(), index);
//             send_response(&to_gui, format!("Marked index {} for deletion.", index));
//         }
//         Command::Create(path, is_file) => {
//             kernel.lock().unwrap().create(current_node.clone(), path.clone(), is_file);
//             send_response(&to_gui, format!("Marked {} for creation.", path));
//         }
//         Command::Move(original_path, new_path) => {
//             kernel.lock().unwrap().move_item(original_path.clone(), new_path.clone());
//             send_response(&to_gui, format!("Marked {} to move to {}.", original_path, new_path));
//         }
//         Command::Undo(index) => {
//             kernel.lock().unwrap().undo_deletion(index);
//             send_response(&to_gui, format!("Unmarked index {} for deletion.", index));
//         }
//         Command::Display => {
//             let display = kernel.lock().unwrap().display(current_node.clone());
//             send_response(&to_gui, display);
//         }
//         Command::Up => {
//             if let Some(parent) = kernel.lock().unwrap().get_parent(current_node.clone()) {
//                 let upgraded = parent.upgrade().expect("Error unwrapping parent upgrade");
//                 *current_node = upgraded;
//                 send_response(&to_gui, "Moved up to parent directory.".to_string());
//             } else {
//                 send_error(&to_gui, "Already at the root directory.".to_string());
//             }
//         }
//         Command::Down(index) => {
//             if let Some(child_node) = kernel.lock().unwrap().get_child(current_node.clone(), index) {
//                 *current_node = child_node;
//                 send_response(to_gui, "Navigated to child directory.".to_string());
//             } else {
//                 send_error(to_gui, "Invalid child index.".to_string());
//             }
//         }
//         Command::Commit => {
//             kernel.lock().unwrap().commit_actions();
//             send_response(to_gui, "Committed deletions".to_string());
//         }
//         Command::Status => {
//             let status = kernel.lock().unwrap().get_status();
//             send_response(to_gui, status);
//         }
//         Command::GoTo(path) => {
//             if let Some(node) = kernel.lock().unwrap().go_to(path.to_string()) {
//                 *current_node = node;
//                 send_response(to_gui, format!("Navigated to {}", path).to_string());
//             } else {
//                 send_error(to_gui, "Invalid path.".to_string());
//             }
//         }
//         Command::Open(index) => {
//             kernel.lock().unwrap().open_file(current_node.clone(), index);
//         }
//         // Handle other commands (Down, Status, Commit, etc.)
//         Command::Exit => {
//             send_response(&to_gui, "Exiting backend.".to_string());
//         }
//         _ => {
//             send_error(&to_gui, "Command not implemented.".to_string());
//         }
//     }
// }


// // fn run_backend(from_gui: mpsc::Receiver<Command>, to_gui: mpsc::Sender<BackendResponse>) {
// //     let visited_inodes: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
// //     let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
// //     let mut fs_root = None;
// //     let mut kernel = None;
// //     let mut current_node = None;

// //     loop {
// //         match from_gui.recv() {
// //             Ok(command) => match command {
// //                 Command::LoadDirectory(path) => {
// //                     if fs_root.is_none() {
// //                         fs_root = FileSystemNode::build_fs_model(
// //                             &path,
// //                             Arc::clone(&visited_inodes),
// //                             small_file_threshold,
// //                             None,
// //                         );
// //                         if let Some(ref root) = fs_root {
// //                             current_node = Some(root.clone());
// //                             kernel = Kernel::new(root.clone());
// //                             to_gui
// //                                 .send(BackendResponse::Response(format!(
// //                                     "Directory loaded: {}",
// //                                     path
// //                                 )))
// //                                 .unwrap();
// //                         } else {
// //                             to_gui
// //                                 .send(BackendResponse::Error(
// //                                     "Failed to load directory.".to_string(),
// //                                 ))
// //                                 .unwrap();
// //                         }
// //                     } else {
// //                         to_gui
// //                             .send(BackendResponse::Error(
// //                                 "Directory already loaded.".to_string(),
// //                             ))
// //                             .unwrap();
// //                     }
// //                 }
// //                 Command::Del(index) => {
// //                     if let Some(kernel) = &mut kernel {
// //                         if let Some(current_node) = &current_node {
// //                             kernel.mark_for_deletion(current_node.clone(), index);
// //                             to_gui
// //                                 .send(BackendResponse::Response(format!(
// //                                     "Marked index {} for deletion.",
// //                                     index
// //                                 )))
// //                                 .unwrap();
// //                         } else {
// //                             to_gui
// //                                 .send(BackendResponse::Error(
// //                                     "No directory loaded.".to_string(),
// //                                 ))
// //                                 .unwrap();
// //                         }
// //                     } else {
// //                         to_gui
// //                             .send(BackendResponse::Error(
// //                                 "Kernel not initialized.".to_string(),
// //                             ))
// //                             .unwrap();
// //                     }
// //                 }
// //                 Command::Commit => {

// //                 }
// //                 Command::Display => {
// //                     if let Some(kernel) = &kernel {
// //                         let display = kernel.display(fs_root.clone().unwrap());
// //                         to_gui
// //                             .send(BackendResponse::Response(display.to_string()))
// //                             .unwrap();
// //                     } else {
// //                         to_gui
// //                             .send(BackendResponse::Error(
// //                                 "Nothing to display; no directory loaded.".to_string(),
// //                             ))
// //                             .unwrap();
// //                     }
// //                 }
// //                 Command::Up => {
// //                     if let Some(parent) = kernel.get_parent(current_node.clone()) {
// //                         current_node = parent.upgrade().expect("Error unwrapping parent upgrade");
// //                     }
// //                 }
// //                 Command::Down(index) => {

// //                 }
// //                 Command::Status => {

// //                 }
// //                 Command::Open(index) => {

// //                 }
// //                 Command::GoTo(index) => {

// //                 }
// //                 Command::Exit => {
// //                     to_gui.send(BackendResponse::Response("Exiting backend.".to_string())).unwrap();
// //                     break;
// //                 }
// //                 Command::Error(message) => {

// //                 }
// //             },
// //             Err(_) => {
// //                 // Handle the sender being disconnected
// //                 break;
// //             }
// //         }
// //     }
// // }


// // pub fn handle_job(root_path: &str) {
// //     // let root_path = "/Users/benjaminxu/Desktop";
// //     let visited_inodes: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
// //     let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
// //     if let Some(fs_root) = FileSystemNode::build_fs_model(root_path, visited_inodes, small_file_threshold, None) {
// //         if let Some(mut kernel) = Kernel::new(fs_root.clone()) {    
// //             let mut current_node = fs_root;
// //             let mut redisplay = true;
// //             loop {
// //                 if redisplay {
// //                     kernel.display(current_node.clone());
// //                 }
// //                 let mut input = String::new();
// //                 std::io::stdin().read_line(&mut input).expect("Failed to read input");
// //                 let input = input.trim();
        
// //                 if input == "exit" {
// //                     break;
// //                 } else if input == ".." {
// //                     if let Some(parent) = kernel.get_parent(current_node.clone()) {
// //                         current_node = parent.upgrade().expect("Error unwrapping parent upgrade");
// //                     }
// //                     redisplay = true;
// //                 } else if input == "commit" {
// //                     kernel.commit_deletions();
// //                     redisplay = false;
// //                 } else if input.starts_with("del ") {
// //                     if let Ok(index) = input[4..].trim().parse::<usize>() {
// //                         kernel.mark_for_deletion(current_node.clone(), index);
// //                         kernel.display(current_node.clone());
// //                     } else {
// //                         println!("Invalid input for del command.");
// //                     }
// //                     redisplay = true;
// //                 } else if input == "status" {
// //                   let status = kernel.get_status();
// //                   println!("{}\n", status);  
// //                   redisplay = false;
// //                 } else if let Ok(index) = input.parse::<usize>() {
// //                     if let Some(child_node) = kernel.get_child(current_node.clone(), index) {
// //                         current_node = child_node;
// //                     }
// //                     redisplay = true;
// //                 } else if input.starts_with("open ") {
// //                     if let Ok(index) = input[5..].trim().parse::<usize>() {
// //                         kernel.open_file(current_node.clone(), index);
// //                     } else {
// //                         println!("Invalid input for del command.");
// //                     }
// //                     redisplay = false;
// //                 } else if input.starts_with("go to ") {
// //                     if let Ok(path) = input[6..].trim().parse::<String>() {
// //                         if let Some(node) = kernel.go_to(path) {
// //                             current_node = node;
// //                             redisplay = true;
// //                         } else {
// //                             println!("Invalid path.");
// //                             redisplay = false;
// //                         }
// //                     } else {
// //                         println!("Invalid input for go to command.");
// //                         redisplay = false;
// //                     }
// //                 } else {
// //                     println!("Invalid input.");
// //                 }
// //             }
// //         } else {

// //         }
// //     } else {
// //         eprintln!("Error reading file system.");
// //     }
// // }
use rayon::prelude::*;
use rayon::current_num_threads;
use std::time::Instant;
use std::ffi::CStr;
use walkdir::WalkDir;
use std::process::Command;
use std::os::raw::c_void;
use libc::*;
use std::mem;


fn fetch_file_system_with_walkdir(path: &str) -> Vec<(String, u64, bool)> {
    WalkDir::new(path)
        .into_iter()
        .par_bridge() // Parallelize the iterator
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let is_file = metadata.is_file();
            let size = if is_file { metadata.len() } else { 0 };
            Some((entry.path().to_string_lossy().to_string(), size, is_file))
        })
        .collect()
}

fn fetch_file_system_with_exec(path: &str) -> Vec<(String, u64, bool)> {
    let output = Command::new("find")
        .arg(path)
        .arg("-ls")
        .output()
        .expect("Failed to execute find");

    if !output.status.success() {
        eprintln!("Find command failed: {:?}", output.status);
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 11 {
                return None;
            }
            let size = parts[6].parse::<u64>().ok()?;
            let path = parts[10..].join(" ");
            let is_file = !path.ends_with('/');
            Some((path, size, is_file))
        })
        .collect()
}

// const VDIR: u32 = 1; // Directory
const VREG: u32 = 1; // Regular file

fn main() {

    let path = "/";

    println!("Benchmarking getattrlistbulk...");
    let start = Instant::now();
    let getattrlistbulk_results = fetch_file_system_with_getattrlistbulk_parallel(path);
    let getattrlistbulk_duration = start.elapsed();
    println!("GetAttrListBulk Duration: {:?}", getattrlistbulk_duration);

    // // Benchmark WalkDir
    // println!("Benchmarking WalkDir...");
    // let start = Instant::now();
    // let walkdir_results = fetch_file_system_with_walkdir(path);
    // let walkdir_duration = start.elapsed();
    // println!("WalkDir Duration: {:?}", walkdir_duration);

    // // Benchmark find
    // println!("Benchmarking find...");
    // let start = Instant::now();
    // let find_results = fetch_file_system_with_exec(path);
    // let find_duration = start.elapsed();
    // println!("Find Duration: {:?}", find_duration);

    // Benchmark getattrlistbulk

    // // Validate results
    // println!("WalkDir Results Count: {}", walkdir_results.len());

    // let mut sum = 0;
    // for r in walkdir_results {
    //     sum += r.1;
    // }
    // println!("Find Results Count: {}", find_results.len());
    println!("GetAttrListBulk Results Count: {}", getattrlistbulk_results.len());

    let mut gsum: u128 = 0;
    for r in getattrlistbulk_results {
        gsum += r.1 as u128;
    }

    println!(" {}", gsum);
}

fn compare_results(walkdir_results: Vec<String>, getattrlistbulk_results: Vec<(String, u64, bool)>) {
    let walkdir_paths: HashSet<_> = walkdir_results.into_iter().collect();
    let getattrlistbulk_paths: HashSet<_> = getattrlistbulk_results.into_iter().map(|(p, _, _)| p).collect();

    let only_in_walkdir = walkdir_paths.difference(&getattrlistbulk_paths);
    let only_in_getattrlistbulk = getattrlistbulk_paths.difference(&walkdir_paths);

    println!("Entries only in WalkDir: {:?}", only_in_walkdir);
    println!("Entries only in GetAttrListBulk: {:?}", only_in_getattrlistbulk);
}
