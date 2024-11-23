use std::collections::{HashSet, VecDeque};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
// use rayon::prelude::*;
use std::os::unix::fs::MetadataExt;
use rust::system::FileSystemNode;
use rust::kernel::Kernel;

fn main() {
    let root_path = "/Users/benjaminxu/Desktop"; // Start from the current directory
    let visited_inodes: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
    let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
    if let Some(fs_root) = FileSystemNode::build_fs_model(root_path, visited_inodes, small_file_threshold, None) {
        if let Some(mut kernel) = Kernel::new(fs_root.clone()) {    
            let mut current_node = fs_root;
            let mut redisplay = true;
            loop {
                if redisplay {
                    kernel.display(current_node.clone());
                }
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).expect("Failed to read input");
                let input = input.trim();
        
                if input == "exit" {
                    break;
                } else if input == ".." {
                    if let Some(parent) = kernel.get_parent(current_node.clone()) {
                        current_node = parent.upgrade().expect("Error unwrapping parent upgrade");
                    }
                    redisplay = true;
                } else if input == "commit" {
                    kernel.commit_deletions();
                    redisplay = false;
                } else if input.starts_with("del ") {
                    if let Ok(index) = input[4..].trim().parse::<usize>() {
                        kernel.mark_for_deletion(current_node.clone(), index);
                        kernel.display(current_node.clone());
                    } else {
                        println!("Invalid input for del command.");
                    }
                    redisplay = true;
                } else if input == "status" {
                  let status = kernel.get_status();
                  println!("{}\n", status);  
                  redisplay = false;
                } else if let Ok(index) = input.parse::<usize>() {
                    if let Some(child_node) = kernel.get_child(current_node.clone(), index) {
                        current_node = child_node;
                    }
                    redisplay = true;
                } else if input.starts_with("open ") {
                    if let Ok(index) = input[5..].trim().parse::<usize>() {
                        kernel.open_file(current_node.clone(), index);
                    } else {
                        println!("Invalid input for del command.");
                    }
                    redisplay = false;
                } else if input.starts_with("go to ") {
                    if let Ok(path) = input[6..].trim().parse::<String>() {
                        if let Some(node) = kernel.go_to(path) {
                            current_node = node;
                            redisplay = true;
                        } else {
                            println!("Invalid path.");
                            redisplay = false;
                        }
                    } else {
                        println!("Invalid input for go to command.");
                        redisplay = false;
                    }
                } else {
                    println!("Invalid input.");
                }
            }
        } else {

        }
    } else {
        eprintln!("Error reading file system.");
    }
}
