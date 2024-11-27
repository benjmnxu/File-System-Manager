use std::collections::{HashSet, VecDeque};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
// use rayon::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::sync::mpsc;
use std::thread;

use rust::system::FileSystemNode;
use rust::kernel::Kernel;
use rust::gui;
use rust::threads::*;

fn main() {
    let (to_backend, from_gui) = mpsc::channel();
    let (to_gui, from_backend) = mpsc::channel();

    let backend_thread = thread::spawn(move || {
        run_backend(&from_gui, &to_gui);
    });

    // Start the GUI application in the main thread
    gui::run_app(to_backend, from_backend).unwrap();

    // Wait for the backend thread to finish
    backend_thread.join().unwrap();
}

enum BackendState {
    Uninitialized,
    Initialized {
        fs_root: Rc<RefCell<FileSystemNode>>,
        kernel: Rc<RefCell<Kernel>>,
        current_node: Rc<RefCell<FileSystemNode>>,
    },
}

fn run_backend(from_gui: &mpsc::Receiver<Command>, to_gui: &mpsc::Sender<BackendResponse>) {
    let visited_inodes: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
    let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
    let mut state = BackendState::Uninitialized;

    loop {
        match from_gui.recv() {
            Ok(command) => match &mut state {
                BackendState::Uninitialized => match command {
                    Command::LoadDirectory(path) => {
                        match FileSystemNode::build_fs_model(
                            &path,
                            Arc::clone(&visited_inodes),
                            small_file_threshold,
                            None,
                        ) {
                            Some(fs_root) => {
                                let root = fs_root;
                                let kernel = Rc::new(RefCell::new(Kernel::new(root.clone())));
                                state = BackendState::Initialized {
                                    fs_root: root.clone(),
                                    kernel,
                                    current_node: root,
                                };
                                send_response(&to_gui, format!("Directory loaded: {}", path));
                            }
                            None => send_error(&to_gui, "Failed to load directory.".to_string()),
                        }
                    }
                    Command::Exit => {
                        send_response(&to_gui, "Exiting backend.".to_string());
                        break;
                    }
                    _ => {
                        send_error(&to_gui, "Load a directory before issuing commands.".to_string());
                    }
                },
                BackendState::Initialized {
                    fs_root,
                    kernel,
                    current_node,
                } => {
                    handle_command(command, fs_root.clone(), kernel.clone(), current_node, &to_gui);
                }
            },
            Err(_) => break, // Handle sender disconnect
        }
    }
}

fn handle_command(
    command: Command,
    fs_root: Rc<RefCell<FileSystemNode>>,
    kernel: Rc<RefCell<Kernel>>,
    current_node: &mut Rc<RefCell<FileSystemNode>>,
    to_gui: &mpsc::Sender<BackendResponse>,
) {
    match command {
        Command::Del(index) => {
            kernel.borrow_mut().mark_for_deletion(current_node.clone(), index);
            send_response(&to_gui, format!("Marked index {} for deletion.", index));
        }
        Command::Display => {
            let display = kernel.borrow().display(fs_root.clone());
            send_response(&to_gui, display);
        }
        Command::Up => {
            if let Some(parent) = kernel.borrow().get_parent(current_node.clone()) {
                let upgraded = parent.upgrade().expect("Error unwrapping parent upgrade");
                *current_node = upgraded;
                send_response(&to_gui, "Moved up to parent directory.".to_string());
            } else {
                send_error(&to_gui, "Already at the root directory.".to_string());
            }
        }
        Command::Down(index) => {
            if let Some(child_node) = kernel.borrow().get_child(current_node.clone(), index) {
                *current_node = child_node;
                send_response(to_gui, "Navigated to child directory.".to_string());
            } else {
                send_error(to_gui, "Invalid child index.".to_string());
            }
        }
        Command::Commit => {
            kernel.borrow_mut().commit_deletions();
            send_response(to_gui, "Committed deletions".to_string());
        }
        Command::Status => {
            let status = kernel.borrow().get_status();
            send_response(to_gui, status);
        }
        Command::GoTo(path) => {
            if let Some(node) = kernel.borrow().go_to(path.to_string()) {
                *current_node = node;
                send_response(to_gui, format!("Navigated to {}", path).to_string());
            } else {
                send_error(to_gui, "Invalid path.".to_string());
            }
        }
        Command::Open(index) => {
            kernel.borrow().open_file(current_node.clone(), index);
        }
        // Handle other commands (Down, Status, Commit, etc.)
        Command::Exit => {
            send_response(&to_gui, "Exiting backend.".to_string());
        }
        _ => {
            send_error(&to_gui, "Command not implemented.".to_string());
        }
    }
}


// fn run_backend(from_gui: mpsc::Receiver<Command>, to_gui: mpsc::Sender<BackendResponse>) {
//     let visited_inodes: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
//     let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
//     let mut fs_root = None;
//     let mut kernel = None;
//     let mut current_node = None;

//     loop {
//         match from_gui.recv() {
//             Ok(command) => match command {
//                 Command::LoadDirectory(path) => {
//                     if fs_root.is_none() {
//                         fs_root = FileSystemNode::build_fs_model(
//                             &path,
//                             Arc::clone(&visited_inodes),
//                             small_file_threshold,
//                             None,
//                         );
//                         if let Some(ref root) = fs_root {
//                             current_node = Some(root.clone());
//                             kernel = Kernel::new(root.clone());
//                             to_gui
//                                 .send(BackendResponse::Response(format!(
//                                     "Directory loaded: {}",
//                                     path
//                                 )))
//                                 .unwrap();
//                         } else {
//                             to_gui
//                                 .send(BackendResponse::Error(
//                                     "Failed to load directory.".to_string(),
//                                 ))
//                                 .unwrap();
//                         }
//                     } else {
//                         to_gui
//                             .send(BackendResponse::Error(
//                                 "Directory already loaded.".to_string(),
//                             ))
//                             .unwrap();
//                     }
//                 }
//                 Command::Del(index) => {
//                     if let Some(kernel) = &mut kernel {
//                         if let Some(current_node) = &current_node {
//                             kernel.mark_for_deletion(current_node.clone(), index);
//                             to_gui
//                                 .send(BackendResponse::Response(format!(
//                                     "Marked index {} for deletion.",
//                                     index
//                                 )))
//                                 .unwrap();
//                         } else {
//                             to_gui
//                                 .send(BackendResponse::Error(
//                                     "No directory loaded.".to_string(),
//                                 ))
//                                 .unwrap();
//                         }
//                     } else {
//                         to_gui
//                             .send(BackendResponse::Error(
//                                 "Kernel not initialized.".to_string(),
//                             ))
//                             .unwrap();
//                     }
//                 }
//                 Command::Commit => {

//                 }
//                 Command::Display => {
//                     if let Some(kernel) = &kernel {
//                         let display = kernel.display(fs_root.clone().unwrap());
//                         to_gui
//                             .send(BackendResponse::Response(display.to_string()))
//                             .unwrap();
//                     } else {
//                         to_gui
//                             .send(BackendResponse::Error(
//                                 "Nothing to display; no directory loaded.".to_string(),
//                             ))
//                             .unwrap();
//                     }
//                 }
//                 Command::Up => {
//                     if let Some(parent) = kernel.get_parent(current_node.clone()) {
//                         current_node = parent.upgrade().expect("Error unwrapping parent upgrade");
//                     }
//                 }
//                 Command::Down(index) => {

//                 }
//                 Command::Status => {

//                 }
//                 Command::Open(index) => {

//                 }
//                 Command::GoTo(index) => {

//                 }
//                 Command::Exit => {
//                     to_gui.send(BackendResponse::Response("Exiting backend.".to_string())).unwrap();
//                     break;
//                 }
//                 Command::Error(message) => {

//                 }
//             },
//             Err(_) => {
//                 // Handle the sender being disconnected
//                 break;
//             }
//         }
//     }
// }


// pub fn handle_job(root_path: &str) {
//     // let root_path = "/Users/benjaminxu/Desktop";
//     let visited_inodes: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
//     let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
//     if let Some(fs_root) = FileSystemNode::build_fs_model(root_path, visited_inodes, small_file_threshold, None) {
//         if let Some(mut kernel) = Kernel::new(fs_root.clone()) {    
//             let mut current_node = fs_root;
//             let mut redisplay = true;
//             loop {
//                 if redisplay {
//                     kernel.display(current_node.clone());
//                 }
//                 let mut input = String::new();
//                 std::io::stdin().read_line(&mut input).expect("Failed to read input");
//                 let input = input.trim();
        
//                 if input == "exit" {
//                     break;
//                 } else if input == ".." {
//                     if let Some(parent) = kernel.get_parent(current_node.clone()) {
//                         current_node = parent.upgrade().expect("Error unwrapping parent upgrade");
//                     }
//                     redisplay = true;
//                 } else if input == "commit" {
//                     kernel.commit_deletions();
//                     redisplay = false;
//                 } else if input.starts_with("del ") {
//                     if let Ok(index) = input[4..].trim().parse::<usize>() {
//                         kernel.mark_for_deletion(current_node.clone(), index);
//                         kernel.display(current_node.clone());
//                     } else {
//                         println!("Invalid input for del command.");
//                     }
//                     redisplay = true;
//                 } else if input == "status" {
//                   let status = kernel.get_status();
//                   println!("{}\n", status);  
//                   redisplay = false;
//                 } else if let Ok(index) = input.parse::<usize>() {
//                     if let Some(child_node) = kernel.get_child(current_node.clone(), index) {
//                         current_node = child_node;
//                     }
//                     redisplay = true;
//                 } else if input.starts_with("open ") {
//                     if let Ok(index) = input[5..].trim().parse::<usize>() {
//                         kernel.open_file(current_node.clone(), index);
//                     } else {
//                         println!("Invalid input for del command.");
//                     }
//                     redisplay = false;
//                 } else if input.starts_with("go to ") {
//                     if let Ok(path) = input[6..].trim().parse::<String>() {
//                         if let Some(node) = kernel.go_to(path) {
//                             current_node = node;
//                             redisplay = true;
//                         } else {
//                             println!("Invalid path.");
//                             redisplay = false;
//                         }
//                     } else {
//                         println!("Invalid input for go to command.");
//                         redisplay = false;
//                     }
//                 } else {
//                     println!("Invalid input.");
//                 }
//             }
//         } else {

//         }
//     } else {
//         eprintln!("Error reading file system.");
//     }
// }