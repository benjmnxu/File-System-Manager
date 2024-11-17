use std::collections::{HashSet, VecDeque};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
// use rayon::prelude::*;
use std::os::unix::fs::MetadataExt; // For Unix-based systems
use rust::system::FileSystemNode;
use rust::kernel::Kernel;

fn main() {
    let root_path = "/Users/benjaminxu/Desktop/things"; // Start from the current directory
    let visited_inodes: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
    let small_file_threshold = 1024; // Define threshold in bytes (e.g., 1 KB)
    if let Some(fs_root) = FileSystemNode::build_fs_model(root_path, visited_inodes, small_file_threshold, None) {
        if let Some(mut kernel) = Kernel::new() {    
            let root = fs_root.clone();
            let mut current_node = root;
            loop {
                kernel.display(current_node.clone());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).expect("Failed to read input");
                let input = input.trim();
        
                if input == "exit" {
                    break;
                } else if input == ".." {
                    if let Some(parent) = kernel.get_parent(current_node.clone()) {
                        current_node = parent.upgrade().expect("Error unwrapping parent upgrade");
                    }
                } else if input == "commit" {
                    kernel.commit_deletions();
                } else if input.starts_with("mark ") {
                    if let Ok(index) = input[5..].trim().parse::<usize>() {
                        kernel.mark_for_deletion(current_node.clone(), index);
                        kernel.display(current_node.clone());
                    } else {
                        println!("Invalid input for mark command.");
                    }
                } else if let Ok(index) = input.parse::<usize>() {
                    if let Some(child_node) = kernel.get_child(current_node.clone(), index) {
                        current_node = child_node;
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
