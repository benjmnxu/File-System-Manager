use std::collections::{HashSet, VecDeque};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::os::unix::fs::MetadataExt; // For Unix-based systems

#[derive(Debug)]
pub struct FileSystemNode {
    name: String,
    path: String,
    is_file: bool,
    size: u64,
    parent: Option<Weak<RefCell<FileSystemNode>>>, // Use Rc<RefCell<>> for parent
    children: Vec<Rc<RefCell<FileSystemNode>>>,  // Use Rc<RefCell<>> for children
    to_be_deleted: bool,
}

impl FileSystemNode {

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn get_parent(&self) -> Option<Weak<RefCell<FileSystemNode>>> {
        self.parent.clone()
    }

    // pub fn get_children(&self) -> Vec<Rc<RefCell<FileSystemNode>>> {
    //     self.children
    // }
    pub fn for_each_child<F>(&self, function: F)
    where
        F: Fn(usize, &Rc<RefCell<FileSystemNode>>),
    {
        let mut result = Vec::new();
        for (i, child) in self.children.iter().enumerate() {
            result.push(function(i, child));
        }
    }

    pub fn get_child(&self, index: usize) -> Option<Rc<RefCell<FileSystemNode>>> { 
        if index >= self.children.len() {
            return None;
        }

        let child = self.children[index].clone();
        Some(child)
    }

    pub fn go_to(&self, name: &str) -> Option<Rc<RefCell<FileSystemNode>>> {
        for child in &self.children {
            if child.borrow().get_name() == name {
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


    pub fn build_fs_model<P: AsRef<Path>>(
        path: P,
        visited_inodes: Arc<Mutex<HashSet<u64>>>,
        small_file_threshold: u64, // Size threshold in bytes
        parent: Option<Weak<RefCell<FileSystemNode>>>
    ) -> Option<Rc<RefCell<Self>>> {
        let path = path.as_ref();
        let metadata = fs::symlink_metadata(path).ok()?;
        let is_file = metadata.is_file();
        let inode = metadata.ino(); // Get inode number (for Unix-based systems)

        {
            let mut visited = visited_inodes.lock().unwrap();
            if visited.contains(&inode) {
                return None;
            }
            visited.insert(inode);
        }

        let name = path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/".to_string());

        let node = Rc::new(RefCell::new(Self {
            name: name.clone(),
            path: path.display().to_string(),
            is_file,
            size: metadata.len(),
            parent: parent.clone(),
            children: vec![],
            to_be_deleted: false,
        }));

        if is_file {
            return Some(node);
        }

        let entries = fs::read_dir(path).ok()?;
        let mut total_size = 0;
        let mut children = vec![];

        for entry in entries.filter_map(|e| e.ok()) {
            let child_path = entry.path();
            if let Some(child_node) = Self::build_fs_model(
                child_path,
                Arc::clone(&visited_inodes),
                small_file_threshold,
                Some(Rc::downgrade(&node))
            ) {
                total_size += child_node.borrow().size;
                children.push(child_node);
            }
        }

        let mut node_mut = node.borrow_mut();
        node_mut.size = total_size;
        node_mut.children = children;
        drop(node_mut);

        Some(node)
    }
}

pub fn disown(node: Rc<RefCell<FileSystemNode>>) {
    // Remove self from parent's children if it exists
    if let Some(parent_weak) = node.borrow().parent.as_ref() {
        if let Some(parent_rc) = parent_weak.upgrade() {
            let mut parent_ref = parent_rc.borrow_mut();
            // Find and remove self from parent's children
            if let Some(index) = parent_ref.children.iter().position(|child| Rc::ptr_eq(child, &node)) {
                parent_ref.size -= node.borrow().size();
                parent_ref.children.remove(index);
            }
        }
    }
}

pub fn prune(node: Rc<RefCell<FileSystemNode>>) {
    // Recursively prune children
    {
        let children = node.borrow_mut().children.drain(..).collect::<Vec<_>>();
        for child in children {
            prune(child);
        }
    }

    disown(node);
}

