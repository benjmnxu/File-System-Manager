use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

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

    pub fn set_size(&mut self, size: u64) {
        self.size = size;
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

    pub fn remove_child(&mut self, path: String) {
        // Find the index of the child to remove
        let mut index = -1;
        let mut i = 0;
        while i < self.children_len() {
            let s = self.children[i].lock().unwrap().get_path().to_string_lossy().to_string();
            if s == path {
                index = i as i16;
                break;
            }
            i+=1;
        }

        // Remove the child if found
        if index != -1{
            let size = self.children[index as usize].lock().unwrap().size();
            self.children.remove(index as usize);
            self.size -= size;
        } else {
            println!("Child with path '{}' not found.", path);
        }
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
    // Lock the node once to access its parent and path
    let (parent_weak, node_path) = {
        let node_ref = node.lock().unwrap();
        (node_ref.parent.clone(), node_ref.get_path().to_string_lossy().to_string())
    };

    // If the parent exists, remove the node from the parent's children
    if let Some(parent_rc) = parent_weak.and_then(|weak| weak.upgrade()) {
        let mut parent_ref = parent_rc.lock().unwrap();
        parent_ref.remove_child(node_path);
    }
}

pub fn prune(node: Arc<Mutex<FileSystemNode>>) {
    // Recursively prune children
    let children = node.lock().unwrap().children.drain(..).collect::<Vec<_>>();
    for child in children {
        prune(child);
    }
    
}

pub async fn build_fs_model(path: String) -> Option<Arc<Mutex<FileSystemNode>>> {
    // Initialize a map to store nodes by path
    let mut nodes: HashMap<String, Arc<Mutex<FileSystemNode>>> = HashMap::new();
    let root = Arc::new(Mutex::new(FileSystemNode {
        name: path.clone(),
        path: PathBuf::from(path.clone()),
        is_file: false,
        size: 0,
        parent: None,
        children: vec![],
        to_be_deleted: false,
    }));

    nodes.insert(path.clone(), root.clone());
    // Fetch the filesystem structure in parallel
    let mut all_results = Vec::new();
    let results = mac::fetch_file_system_with_getattrlistbulk_parallel(&path);
    all_results.extend(results);
    

    // Populate nodes map with results
    for (entry_path, size, is_file) in all_results {
        let path_buf = PathBuf::from(&entry_path);
        let name = path_buf
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/".to_string());

        let node = Arc::new(Mutex::new(FileSystemNode {
            name,
            path: path_buf.clone(),
            is_file,
            size,
            parent: None,
            children: vec![],
            to_be_deleted: false,
        }));

        nodes.insert(entry_path.clone(), node);
    }

    // Link children to their parent nodes
    for (entry_path, node) in &nodes {
        let path_buf = PathBuf::from(entry_path);
        if let Some(parent_path) = path_buf.parent() {
            let parent_path_str = parent_path.to_string_lossy().to_string();
            if let Some(parent_node) = nodes.get(&parent_path_str) {
                let mut parent_lock = parent_node.lock().unwrap();
                let mut node_lock = node.lock().unwrap();
                node_lock.parent = Some(Arc::downgrade(parent_node));
                parent_lock.children.push(Arc::clone(node));
            }
        }
    }

    // Aggregate directory sizes
    populate_size(root.clone());
    Some(root)
}

pub fn populate_size(node: Arc<Mutex<FileSystemNode>>) {
    let mut total_size = node.lock().unwrap().size;

    // Lock the node once to get the children
    let children: Vec<Arc<Mutex<FileSystemNode>>> = {
        let locked_node = node.lock().unwrap();
        locked_node.children.clone() // Clone the child references to avoid holding the lock
    };

    // Recursively populate size for each child
    for child in children {
        populate_size(child.clone());
        total_size += child.lock().unwrap().size; // Accumulate the size
    }

    // Lock the node again to update its size
    {
        let mut locked_node = node.lock().unwrap();
        locked_node.size = total_size;
    }
}