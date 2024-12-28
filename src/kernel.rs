use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};

use crate::system::{disown, prune, FileSystemNode};
use crate::ai;

enum KernelAction {
    CREATE {
        path: PathBuf,
        is_file: bool
    },
    DELETE {
        target:Arc<Mutex<FileSystemNode>>
    },
    MOVE {
        original_path: String,
        new_path: String
    },
}

pub struct Kernel {
    root: Arc<Mutex<FileSystemNode>>,
    // marked_for_deletion: Vec<Rc<Mutex<FileSystemNode>>>,
    actions: VecDeque<KernelAction>,
    ai_suggestion: String,
    action_file: bool,
    dry:bool
}

impl Kernel {

    pub fn new(root: Arc<Mutex<FileSystemNode>>, action_file: bool, dry: bool) -> Self {
        Kernel {
            root: root.clone(),
            // marked_for_deletion: Vec::new(),
            actions: VecDeque::new(),
            ai_suggestion: String::new(),
            action_file,
            dry
        }
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


    pub fn display(&self, node: Arc<Mutex<FileSystemNode>>) -> String {
        let borrowed = node.lock().unwrap();
        let mut display = format!("\nCurrent Directory: {}\n", borrowed.get_path().to_string_lossy());
    
        borrowed.for_each_child(|i, child| {
            let child_node = child.lock().unwrap();
            if !child_node.is_marked() {
                let node_type = if child_node.is_file() { "[File]" } else { "[Directory]" };
                display.push_str(&format!(
                    "[{}] {} ({} {})\n",
                    i,
                    child_node.get_name(),
                    self.format_size(child_node.size()),
                    node_type
                ));
            }
        });
    
        display.push_str(&format!(
            "Total storage used: {}\n",
            self.format_size(borrowed.size())
        ));
    
        display
    }
    

    pub fn get_parent(&self, node: Arc<Mutex<FileSystemNode>>) -> Option<Weak<Mutex<FileSystemNode>>> {
        node.lock().unwrap().get_parent()
    }


    pub fn get_child(&self, node: Arc<Mutex<FileSystemNode>>, index: usize) -> Option<Arc<Mutex<FileSystemNode>>> {
        let borrowed = node.lock().unwrap();
        let child = borrowed.get_child(index);

        match &child {
            Some(ref node) => {
                if node.lock().unwrap().is_file() {
                    println!("Cannot navigate into a file.");
                    return None;
                }
            },
            None => println!("Invalid index."),
        }
        child
    }

    pub fn get_index(&self, node: Arc<Mutex<FileSystemNode>>, child: String) -> Option<usize> {
        let borrowed = node.lock().unwrap();
        let mut i = 0;
        let l = borrowed.children_len();
        let path = PathBuf::from(child);

        while i < l {
            if let Some(child) = borrowed.get_child(i) {
                if *child.lock().unwrap().get_path() == path {
                    return Some(i);
                }
            }
            i +=1 ;
        }
        None
    }

    pub fn set_suggestion(&mut self, suggestion: String) {
        self.ai_suggestion = suggestion;
    }

    pub fn get_suggestion(&self) -> String {
        self.ai_suggestion.clone()
    }

    pub fn convert_suggestions(&mut self, node: Arc<Mutex<FileSystemNode>>) {
        let commands = ai::parse_ai(&self.ai_suggestion);
        for command in commands {
            match command {
                ai::AICommand::DeleteFile { delete_file } => {
                    let index = self.get_index(node.clone(), delete_file.path);
                    if let Some(i) = index {
                        self.mark_for_deletion(node.clone(), i);
                    }
                }
                ai::AICommand::MoveItem { move_item } => {
                    self.move_item(move_item.original_location, move_item.new_location);
                }
                ai::AICommand::CreateDirectory { create_directory } => {
                    self.create(node.clone(), create_directory.path, false);
                }
            }
        }
    }

    pub fn go_to(&self, mut path: String) -> Option<Arc<Mutex<FileSystemNode>>> {
        let mut current_node = Some(self.root.clone());
    
        // Adjust the path to be relative if it starts with the root's path
        let relative_path = if let Some(root_node) = &current_node {
            let root_str = root_node.lock().unwrap().get_path().to_string_lossy().to_string();
            if path.starts_with(&root_str) {
                path.split_off(root_str.len())
            } else {
                path
            }
        } else {
            path // Default to the original path if no root node is found
        };
    
        // Iterate through the components of the path
        for address in relative_path.split('/').filter(|part| !part.is_empty()) {
            if let Some(node) = current_node {
                current_node = node.lock().unwrap().go_to(address);
            } else {
                return None;
            }
        }
    
        current_node
    }
    

    pub fn get_status(&self) -> String {
        let mut total_space_saved = 0;
        let mut index = 0;
        let status: Vec<String> = self
            .actions
            .iter()
            .map(|item| {
                
                let action = match item {
                    KernelAction::CREATE { path, is_file: _ } => {
                        format!("[{}] CREATE: {}", index, path.to_string_lossy().to_string())
                    }
                    KernelAction::DELETE { target  } => {
                        let borrowed_item = target.lock().unwrap();
                        let path = borrowed_item.get_path().to_string_lossy();
                        total_space_saved += borrowed_item.size();
                        format!("[{}] DELETE: {} ({})", index, path.to_string(), self.format_size(borrowed_item.size()))
                    }
                    KernelAction::MOVE { original_path, new_path } => {
                        format!("[{}] MOVE: {} -> {}", index, original_path.clone(), new_path.clone())
                    }
                };
                index+=1;
                action
            })
            .collect();

        format!(
            "The following are marked for action: \n {} \nTotal space saved: {}",
            status.join("\n"),
            self.format_size(total_space_saved)
        )
    }

    pub fn mark_for_deletion(&mut self, node: Arc<Mutex<FileSystemNode>>, index: usize) {
        if let Some(to_delete) = node.lock().unwrap().get_child(index) {
            // self.marked_for_deletion.push(to_delete.clone());
            self.actions.push_back(KernelAction::DELETE { target: to_delete.clone() });
            to_delete.lock().unwrap().delete();
        }
    }


    pub fn undo_deletion(&mut self, index: usize) {

        match &self.actions[index] {
            KernelAction::DELETE {target} => {
                target.lock().unwrap().undelete();
            }
            _=>{}
        }
        self.actions.remove(index);
    }

    pub fn commit_actions(&mut self) {
        while let Some(action) = self.actions.pop_front() {
            match action {
                KernelAction::CREATE { path, is_file } => {
                    if !self.dry {
                        self.commit_creation(path.clone(), is_file)
                    }
                    if self.action_file {
                        let _ = fs::write("changes.txt", format!("CREATE {}", path.clone().display()));
                    }
                }
                KernelAction::DELETE { target } => {
                    if !self.dry {
                        self.commit_deletion(target.clone());
                    }
                    if self.action_file {
                        let _ = fs::write("changes.txt", format!("DELETE {}", target.lock().unwrap().get_path().display()));
                    }
                }
                KernelAction::MOVE { original_path, new_path } => {
                    if !self.dry {
                        self.commit_move(original_path.clone(), new_path.clone());
                    }
                    if self.action_file {
                        let _ = fs::write("changes.txt", format!("MOVE {} > {}", original_path.clone(), new_path.clone()));
                    }
                }
            }
        }
    }

    pub fn commit_deletion(&mut self, target: Arc<Mutex<FileSystemNode>>) {
        println!("\nCommitting deletions...");
        // if Rc::strong_count(&node) == 1 {
        //     continue;
        // }
        let (path, is_file) = {
            let borrowed_node = target.lock().unwrap();
            (borrowed_node.get_path().clone(), borrowed_node.is_file())
        };

        // Perform file or directory deletion based on the extracted data
        if is_file {
            match fs::remove_file(&path) {
                Ok(_) => { 
                    println!("Deleted file: {}", path.display());
                    disown(target);
                },
                Err(e) => eprintln!("Failed to delete file {}: {}", path.display(), e),
            }
        } else {
            match fs::remove_dir_all(&path) {
                Ok(_) => {
                    println!("Deleted directory: {}", path.display());
                    prune(target.clone());
                    disown(target);
                },
                Err(e) => eprintln!("Failed to delete directory {}: {}", path.display(), e),
            }
        }
    }

    pub fn commit_creation(&mut self, path: PathBuf, is_file: bool) {
        if is_file {
            let _ = fs::write(&path, "");
        } else {
            let _ = fs::create_dir_all(&path);
        }
    }

    pub fn commit_move(&mut self, original_path: String, new_path: String) {
        let _ = fs::rename(original_path, new_path);
    }

    pub fn open_file(&self, node: Arc<Mutex<FileSystemNode>>, index: usize) {
        let child = node.lock().unwrap().get_child(index);
        if let Some(child_node) = child {
            let borrowed_node = child_node.lock().unwrap();
            let file_path = borrowed_node.get_path();
            if cfg!(target_os = "macos") {
                Command::new("open")
                    .arg(file_path)
                    .spawn()
                    .expect("Failed to open file");
            // } else if cfg!(target_os = "windows") {
            //     Command::new("cmd")
            //         .args(&["/C", "start", "", file_path])
            //         .spawn()
            //         .expect("Failed to open file");
            // } else if cfg!(target_os = "linux") {
            //     Command::new("xdg-open")
            //         .arg(file_path)
            //         .spawn()
            //         .expect("Failed to open file");
            } else {
                eprintln!("Unsupported operating system");
            }
        }
    }

    pub fn create(&mut self, node: Arc<Mutex<FileSystemNode>>, path: String, is_file: bool) {
        let mut current = node.clone();
        let mut path_so_far = current.lock().unwrap().get_path().clone();

        if path.starts_with("/") {
            current = self.root.clone();
            path_so_far = PathBuf::new();
        }

        let addresses: Vec<&str> = path.split("/").collect();
        let n = addresses.len();

        let mut i = 0;
        while i < n {
            let address = addresses[i];
            path_so_far.push(address);
            if address == ".." {
                let mut parent = None;
                if let Some(weak_parent) = current.lock().unwrap().get_parent() {
                    if let Some(strong_parent) = weak_parent.upgrade() {
                        parent = Some(strong_parent);
                    }
                }
                if let Some(p) = parent {
                    current = p;
                }
            } else if address != "." {
                let mut child;
                let has_child;
                {
                    child = current.lock().unwrap().go_to(address);
                    has_child = child.is_some();
                }

                if !has_child {
                    let new_node = FileSystemNode::new(addresses[n-1].to_string(), path_so_far.to_path_buf(), i == n - 1 && is_file, 0, Some(Arc::downgrade(&current)), Vec::new(), false);
                    let next = Arc::new(Mutex::new(new_node));
                    current.lock().unwrap().add_child(next.clone());
                    child = Some(next)
                }
                
                if let Some(kid) = child {
                    current = kid.clone();
                }

            }
            i+=1;
        }

        self.actions.push_back(KernelAction::CREATE { path: path_so_far.to_path_buf(), is_file });
    }

    pub fn move_item(&mut self, original_path: String, new_path: String) {
    
        if let Some(node) = self.go_to(original_path.clone()) {
    
            let parent = {
                let node_ref = node.lock().unwrap();
                node_ref.get_parent().and_then(|weak_parent| weak_parent.upgrade())
            };
    
            if let Some(parent_arc) = parent {
                let mut parent_ref = parent_arc.lock().unwrap();
                parent_ref.remove_child(original_path.clone());
    
                let size = parent_ref.size();
                parent_ref.set_size(size - node.lock().unwrap().size());
            } else {
                println!("Parent not found for node with path: {}", original_path);
            }
    
            // Append the item's name to the new path
            let item_name = Path::new(&original_path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "".to_string());
            let final_new_path = Path::new(&new_path)
                .join(item_name.clone())
                .to_string_lossy()
                .into_owned();
    
            if let Some(new_node) = self.go_to(new_path.clone()) {
                new_node.lock().unwrap().add_child(node.clone());
                let size = new_node.lock().unwrap().size();
                new_node.lock().unwrap().set_size(size + node.lock().unwrap().size());
                node.lock().unwrap().set_parent(Some(Arc::downgrade(&new_node)));
            } else {
                println!("New parent not found for path: {}", new_path);
            }
    
            node.lock().unwrap().set_path(final_new_path.clone());
            node.lock().unwrap().set_name(item_name.clone());
    
            // Push the final new path to actions
            self.actions.push_back(KernelAction::MOVE {
                original_path,
                new_path: final_new_path,
            });
        }
    }
    
}