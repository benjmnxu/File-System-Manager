use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, Metadata};
use std::path::{Path, PathBuf};
use std::cell::RefCell;
use std::sync::{Arc, Mutex, Weak};
use std::os::unix::fs::MetadataExt;

use crate::mac;

#[derive(Debug)]
pub struct FileSystemNode {
    name: String,
    path: PathBuf,
    is_file: bool,
    size: u64,
    parent: Option<Weak<Mutex<FileSystemNode>>>,
    children: Vec<Arc<Mutex<FileSystemNode>>>,
    to_be_deleted: bool,
}

impl FileSystemNode {

    pub fn new(
        name: String,
        path: PathBuf,
        is_file: bool,
        size: u64,
        parent: Option<Weak<Mutex<FileSystemNode>>>, // Use Rc<RefCell<>> for parent
        children: Vec<Arc<Mutex<FileSystemNode>>>,  // Use Rc<RefCell<>> for children
        to_be_deleted: bool) -> Self {
            Self {
                name,
                path,
                is_file,
                size,
                parent,
                children,
                to_be_deleted
            }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    pub fn set_path(&mut self, path: String) {
        self.path = PathBuf::from(path);
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn get_parent(&self) -> Option<Weak<Mutex<FileSystemNode>>> {
        self.parent.clone()
    }

    pub fn set_parent(&mut self, parent: Option<Weak<Mutex<FileSystemNode>>>) {
        self.parent = parent;
    }


    pub fn add_child(&mut self, child: Arc<Mutex<FileSystemNode>>){
        self.children.push(child);
    }
    pub fn for_each_child<F>(&self, mut function: F)
    where
        F: FnMut(usize, &Arc<Mutex<FileSystemNode>>),
    {
        for (i, child) in self.children.iter().enumerate() {
            function(i, child);
        }
    }

    pub fn get_child(&self, index: usize) -> Option<Arc<Mutex<FileSystemNode>>> { 
        if index >= self.children.len() {
            return None;
        }

        let child = self.children[index].clone();
        Some(child)
    }

    pub fn remove_child(&self, path: String) {

    }

    pub fn go_to(&self, name: &str) -> Option<Arc<Mutex<FileSystemNode>>> {
        for child in &self.children {
            if child.lock().unwrap().get_name() == name {
                return Some(child.clone());
            }
        }

        None
    }


    pub fn is_marked(&self) -> bool {
        self.to_be_deleted
    }

    pub fn children_len(&self) -> usize {
        self.children.len()
    }

    pub fn delete(&mut self) {
        self.to_be_deleted = true
    }

    pub fn undelete(&mut self) {
        self.to_be_deleted = false
    }
}
pub fn disown(node: Arc<Mutex<FileSystemNode>>) {
    // Remove self from parent's children if it exists
    if let Some(parent_weak) = node.lock().unwrap().parent.as_ref() {
        if let Some(parent_rc) = parent_weak.upgrade() {
            let mut parent_ref = parent_rc.lock().unwrap();
            // Find and remove self from parent's children
            if let Some(index) = parent_ref.children.iter().position(|child| Arc::ptr_eq(child, &node)) {
                parent_ref.size -= node.lock().unwrap().size();
                parent_ref.children.remove(index);
            }
        }
    }
}

pub fn prune(node: Arc<Mutex<FileSystemNode>>) {
    // Recursively prune children
    {
        let children = node.lock().unwrap().children.drain(..).collect::<Vec<_>>();
        for child in children {
            prune(child);
        }
    }

    disown(node);
}

pub async fn build_fs_model(paths: Vec<String>) -> Option<Arc<Mutex<FileSystemNode>>> {
    let mut nodes: HashMap<String, Arc<Mutex<FileSystemNode>>> = HashMap::new();

    for path_str in paths {
        let path = PathBuf::from(&path_str);
        println!("PATH {}", path.display());
        let is_file = path.is_file();
        let mut size = 0;

        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/".to_string());

        let node: Arc<Mutex<FileSystemNode>> = Arc::new(Mutex::new(FileSystemNode {
            name,
            path: path.clone(),
            is_file,
            size,
            parent: None,
            children: vec![],
            to_be_deleted: false
        }));

        nodes.insert(path_str.clone(), node.clone());

        if let Some(parent_path) = path.parent() {
            if let Some(parent_node) = nodes.get(parent_path.to_string_lossy().as_ref()) {
                node.lock().unwrap().parent = Some(Arc::downgrade(parent_node));
                parent_node.lock().unwrap().children.push(node.clone());
            }
        }
    }

    // Calculate directory sizes by summing child sizes
    for node in nodes.values() {
        let mut node_mut = node.lock().unwrap();
        if !node_mut.is_file {
            node_mut.size = node_mut.children.iter()
                .map(|child| child.lock().unwrap().size)
                .sum();
        }
    }

    nodes.get("/").cloned()
}
