use std::fs::{self, File};
use std::path::Path;
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefCell};
use std::collections::{HashSet, VecDeque};

use crate::system::{disown, prune, FileSystemNode};

pub struct Kernel {
    // root: Rc<RefCell<FileSystemNode>>,
    marked_for_deletion: VecDeque<Rc<RefCell<FileSystemNode>>>
}

impl Kernel {

    pub fn new() -> Option<Self> {
        Some(Kernel {
            // root: root.clone(),
            marked_for_deletion: VecDeque::new()
        })
    }

    fn format_size(&self, size: u64) -> String {
        const KIB: u64 = 1024;
        const MIB: u64 = KIB * 1024;
        const GIB: u64 = MIB * 1024;
        const TIB: u64 = GIB * 1024;

        if size >= TIB {
            format!("{:.2} TB", size as f64 / TIB as f64)
        } else if size >= GIB {
            format!("{:.2} GB", size as f64 / GIB as f64)
        } else if size >= MIB {
            format!("{:.2} MB", size as f64 / MIB as f64)
        } else if size >= KIB {
            format!("{:.2} KB", size as f64 / KIB as f64)
        } else {
            format!("{} bytes", size)
        }
    }

    pub fn display(&self, node: Rc<RefCell<FileSystemNode>>) {

        let borrowed = node.borrow();
        println!("\nCurrent Directory: {}", borrowed.get_path());

        borrowed.for_each_child(|i, child| {
            let child_node = child.borrow();
            if !child_node.is_marked() {
                let node_type = if child_node.is_file() { "[File]" } else { "[Directory]" };
                println!(
                    "{}: {} ({} {})",
                    i,
                    child_node.get_name(),
                    self.format_size(child_node.size()),
                    node_type
                );
            }
        });
        

        println!("\nTotal storage used: {}", self.format_size(borrowed.size()));
        println!("\nEnter an index to navigate into a directory, '..' to go up, 'mark <index>' to mark for deletion, 'commit' to delete marked files, or 'exit' to quit.");

    }

    pub fn get_parent(&self, node: Rc<RefCell<FileSystemNode>>) -> Option<Weak<RefCell<FileSystemNode>>> {
        node.borrow().get_parent()
    }

    pub fn get_child(&self, node: Rc<RefCell<FileSystemNode>>, index: usize) -> Option<Rc<RefCell<FileSystemNode>>> {
        let borrowed = node.borrow();
        let child = borrowed.get_child(index);

        match &child {
            Some(ref node) => {
                if node.borrow().is_file() {
                    println!("Cannot navigate into a file.");
                    return None;
                }
            },
            None => println!("Invalid index."),
        }
        child
    }

    pub fn go_to(&self, path: String) {

    }

    pub fn mark_for_deletion(&mut self, node: Rc<RefCell<FileSystemNode>>, index: usize) {
        if let Some(to_delete) = node.borrow_mut().get_child(index) {
            self.marked_for_deletion.push_back(to_delete.clone());
            to_delete.borrow_mut().delete();
        }
    }

    pub fn commit_deletions(&mut self) {
        println!("\nCommitting deletions...");
        while let Some(node) = self.marked_for_deletion.pop_front() {
            if Rc::strong_count(&node) == 1 {
                continue;
            }
            let path;
            let is_file;
            {
                let borrowed_node = node.borrow();
                path = borrowed_node.get_path().to_string();
                is_file = borrowed_node.is_file();
            }

            // Perform file or directory deletion based on the extracted data
            if is_file {
                match fs::remove_file(&path) {
                    Ok(_) => { 
                        println!("Deleted file: {}", path);
                        disown(node);
                    },
                    Err(e) => eprintln!("Failed to delete file {}: {}", path, e),
                }
            } else {
                match fs::remove_dir_all(&path) {
                    Ok(_) => {
                        println!("Deleted directory: {}", path);
                        prune(node);
                    },
                    Err(e) => eprintln!("Failed to delete directory {}: {}", path, e),
                }
            }
        }
    }
}