use std::sync::{Arc, Mutex};
use clap::Parser;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::mpsc;

use rust::system::*;
use rust::kernel::Kernel;
use rust::gui;
use rust::threads::*;
use rust::ai;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable GUI mode
    #[arg(long)]
    gui_mode: bool,

    /// Enable dry run
    #[arg(long)]
    dry: bool,

    // actions
    #[arg(long)]
    action_file: bool
}

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let cli = Cli::parse();
    println!("Parsed CLI arguments: {:?}", cli);
    println!("Dry: {}", cli.dry);
    println!("GUI mode: {}", cli.gui_mode);
    
    if cli.gui_mode {
        let (to_backend, mut from_gui) = mpsc::channel(32);
        let (to_gui, from_backend) = mpsc::channel(32);

        let backend_handle = tokio::spawn(async move {
            run_backend(&mut from_gui, &to_gui, cli.dry, cli.action_file).await;
        });

        // Start the GUI application in the main thread
        gui::run_app(to_backend, from_backend).unwrap();

        // Wait for the backend task to finish
        backend_handle.await.unwrap();
    } else {
        lone_run_backend(cli.dry, cli.action_file).await;
    }
}


enum BackendState {
    Uninitialized,
    Initialized {
        kernel: Arc<Mutex<Kernel>>,
        current_node: Arc<Mutex<FileSystemNode>>,
    },
}

async fn run_backend(from_gui: &mut mpsc::Receiver<Command>, to_gui: &mpsc::Sender<BackendResponse>, dry: bool, action_file: bool) {
    let mut state = BackendState::Uninitialized;

    loop {
        match from_gui.recv().await {
            Some(command) => match &mut state {
                BackendState::Uninitialized => match command {
                    Command::LoadDirectory(path) => {
                        match build_fs_model(path.clone()).await {
                            Some(fs_root) => {
                                let kernel = Arc::new(Mutex::new(Kernel::new(fs_root.clone(), action_file, dry)));
                                state = BackendState::Initialized {
                                    kernel: kernel.clone(),
                                    current_node: fs_root,
                                };
                                send_response(&to_gui, format!("Directory loaded: {}", path)).await;
                            }
                            None => {
                                send_error(&to_gui, "Failed to load directory.".to_string()).await;
                            }
                        }
                    }
                    Command::Exit => break,
                    _ => {
                        send_error(&to_gui, "Load a directory before issuing commands.".to_string()).await;
                    }
                },
                BackendState::Initialized {kernel, current_node } => {
                    let updated_node = handle_command(
                        command,
                        kernel.clone(),
                        current_node.clone(),
                        &to_gui,
                    )
                    .await;
                    *current_node = updated_node;
                }
            },
            None => break, // Handle sender disconnect
        }
    }
}


async fn handle_command(
    command: Command,
    kernel: Arc<Mutex<Kernel>>,
    current_node: Arc<Mutex<FileSystemNode>>,
    to_gui: &mpsc::Sender<BackendResponse>,
) -> Arc<Mutex<FileSystemNode>> {
    println!("Handling command");

    match command {
        Command::Del(index) => {
            {
            let mut kernel_guard = kernel.lock().unwrap();
            kernel_guard.mark_for_deletion(current_node.clone(), index);
            }
            send_response(to_gui, format!("Marked index {} for deletion.", index)).await;
        }
        Command::Create(path, is_file) => {
            {
            let mut kernel_guard = kernel.lock().unwrap();
            
            kernel_guard.create(current_node.clone(), path.clone(), is_file);
            }
            send_response(to_gui, format!("Created {}.", path)).await;
        }
        Command::Move(original_path, new_path) => {
            {
                let mut kernel_guard = kernel.lock().unwrap();
            
                // Get absolute paths
                let abs_original_path = if original_path.starts_with('/') {
                    original_path.to_string()
                } else {
                    let mut abs_path = current_node.lock().unwrap().get_path().to_string_lossy().to_string();
                    abs_path.push('/');
                    abs_path.push_str(&original_path);
                    abs_path
                };
            
                let abs_new_path = if new_path.starts_with('/') {
                    new_path.to_string()
                } else {
                    let mut abs_path = current_node.lock().unwrap().get_path().to_string_lossy().to_string();
                    abs_path.push('/');
                    abs_path.push_str(&new_path);
                    abs_path
                };
            
                // Perform the move
                kernel_guard.move_item(abs_original_path.clone(), abs_new_path.clone());
            }

            println!("Asdf");
        
            // Send response
            send_response(to_gui, format!("Moved {} to {}.", original_path, new_path)).await;
        }
        
        Command::Undo(index) => {
            {let mut kernel_guard = kernel.lock().unwrap();
            kernel_guard.undo_deletion(index);}
            send_response(to_gui, format!("Undid deletion of index {}.", index)).await;
        }
        Command::Display => {
            let display = {
                let kernel_guard = kernel.lock().unwrap();
                kernel_guard.display(current_node.clone())
            };
            send_response(to_gui, display).await;
        }
        Command::Up => {
            let (response, new_node) = {
                let kernel_guard = kernel.lock().unwrap(); // Lock the kernel
                if let Some(parent) = kernel_guard.get_parent(current_node.clone()) {
                    if let Some(upgraded) = parent.upgrade() {
                        ("Moved up to parent directory.".to_string(), Some(upgraded)) // Success case
                    } else {
                        ("Error upgrading parent reference.".to_string(), None) // Failed to upgrade
                    }
                } else {
                    ("Already at the root directory.".to_string(), None) // No parent exists
                }
            }; // End of the scope for the lock
        
            if let Some(new_node) = new_node {
                send_response(to_gui, response).await;
                return new_node; // Return the updated parent node
            } else {
                send_error(to_gui, response).await;
            }
        
        }
        Command::Down(index) => {
            let (response, child) = {
                let kernel_guard = kernel.lock().unwrap();
                if let Some(child_node) = kernel_guard.get_child(current_node.clone(), index) {
                    ("Moved down to child directory.".to_string(), Some(child_node))
                } else {
                    ("Invalid child index.".to_string(), None)
                }
            };

            if let Some(new_node) = child {
                send_response(to_gui, response).await;
                return new_node; // Return the updated parent node
            } else {
                send_error(to_gui, response).await;
            }
        }
        Command::Commit => {
            {let mut kernel_guard = kernel.lock().unwrap();
            kernel_guard.commit_actions();}
            send_response(to_gui, "Committed all actions.".to_string()).await;
        }
        Command::Status => {
            let status = {
                let kernel_guard = kernel.lock().unwrap();
                kernel_guard.get_status()
            };
            send_response(to_gui, status).await;
        }
        Command::GoTo(path) => {
            let (response, node) = {
                let kernel_guard = kernel.lock().unwrap();
                if let Some(node) = kernel_guard.go_to(path.clone()) {
                    (format!("Navigated to {}.", path), Some(node))
                } else {
                    (format!("Invalid path: {}.", path), None)
                }
            };

            if let Some(new_node) = node {
                send_response(to_gui, response).await;
                return new_node;
            } else {
                send_error(to_gui, response).await;
            }

        }
        Command::Open(index) => {
            {let kernel_guard = kernel.lock().unwrap();
            kernel_guard.open_file(current_node.clone(), index);}
            send_response(to_gui, format!("Opened file at index {}.", index)).await;
        }
        Command::Help => {
            let help_message = r#"
        Available Commands:
        1. `..` - Moves up one level.
        2. `<index>` - Moves down to the child at the specified index.
        3. `go to <path>` - Navigates to the specified path.
        4. `commit` - Commits the current state.
        5. `undo <index>` - Reverts to a specific commit index.
        6. `status` - Displays the current status.
        7. `display` - Displays content or structure at the current level.
        8. `create file <name>` - Creates a file with the specified name.
        9. `create folder <name>` - Creates a folder with the specified name.
        10. `del <index>` - Deletes the item at the specified index.
        11. `open <index>` - Opens the item at the specified index.
        12. `move <source> > <destination>` - Moves an item from source to destination.
        13. `help` - Displays this help message.
        "#;
            send_response(to_gui, help_message.to_string()).await;
        }
        Command::AISuggestion(input) => {
            let context = {
                let kernel_guard = kernel.lock().unwrap();
                kernel_guard.display(current_node.clone())
            };
            let response = ai::ask(input, context).await;
            
            let suggestion: String = {
                if let Some(sug) = response {
                    kernel.lock().unwrap().set_suggestion(sug.clone());
                    sug
                } else {
                    "No Suggestions".to_string()
                }
            };

            let _ = to_gui.send(BackendResponse::AIResponse(suggestion)).await;

        }
        Command::AIConfirm => {
            {kernel.lock().unwrap().convert_suggestions(current_node.clone());}

            let _ = to_gui.send(BackendResponse::AIResponse("Applied suggestions (Still needs to be committed)".to_string())).await;
            
        }
        Command::Exit => {
            send_response(to_gui, "Exiting backend.".to_string()).await;
        }
        _ => {
            send_error(to_gui, "Unknown command.".to_string()).await;
        }
    }

    // Return the current node if no change
    current_node
}


async fn lone_run_backend(dry: bool, action_file: bool) {
    let mut state = BackendState::Uninitialized;
    let stdin = tokio::io::stdin(); // Use tokio's async stdin
    let mut reader = BufReader::new(stdin).lines();

    println!("Backend is running. Enter commands:");

    while let Some(input) = reader.next_line().await.unwrap_or_else(|_| None) {
        if input.is_empty() {
            continue; // Skip empty lines
        }

        match &mut state {
            BackendState::Uninitialized => {
                let command = Command::LoadDirectory(input);
                match command {
                    Command::LoadDirectory(path) => {
                        match build_fs_model(path.clone()).await {
                            Some(fs_root) => {
                                let kernel = Arc::new(Mutex::new(Kernel::new(fs_root.clone(), action_file, dry)));
                                state = BackendState::Initialized {
                                    kernel: kernel.clone(),
                                    current_node: fs_root,
                                };
                                println!("Directory loaded: {}", path);
                            }
                            None => {
                                println!("{}", "Failed to load directory.");
                            }
                        }
                    }
                    Command::Exit => {
                        println!("Exiting backend...");
                        break;
                    }
                    _ => {
                        println!("Load a directory before issuing commands.");
                    }
                }
            },
            BackendState::Initialized {
                kernel,
                current_node,
            } => {
                let command = parse_command(&input);
                match command {
                    Command::Exit => {
                        println!("Exiting backend...");
                        break;
                    }
                    _ => {
                        let updated_node = sync_handle_command(
                            command,
                            kernel.clone(),
                            current_node.clone(),
                        )
                        .await;
                        *current_node = updated_node;
                    }
                }
            }
        }
    }
}
async fn sync_handle_command(
    command: Command,
    kernel: Arc<Mutex<Kernel>>,
    current_node: Arc<Mutex<FileSystemNode>>,
) -> Arc<Mutex<FileSystemNode>> {
    println!("Handling command");

    match command {
        Command::Del(index) => {
            {
            let mut kernel_guard = kernel.lock().unwrap();
            kernel_guard.mark_for_deletion(current_node.clone(), index);
            }
            println!("Marked index {} for deletion.", index);
        }
        Command::Create(path, is_file) => {
            {
            let mut kernel_guard = kernel.lock().unwrap();
            
            kernel_guard.create(current_node.clone(), path.clone(), is_file);
            }
            println!("Created {}.", path);
        }
        Command::Move(original_path, new_path) => {
            {
                let mut kernel_guard = kernel.lock().unwrap();
            
                // Get absolute paths
                let abs_original_path = if original_path.starts_with('/') {
                    original_path.to_string()
                } else {
                    let mut abs_path = current_node.lock().unwrap().get_path().to_string_lossy().to_string();
                    abs_path.push('/');
                    abs_path.push_str(&original_path);
                    abs_path
                };
            
                let abs_new_path = if new_path.starts_with('/') {
                    new_path.to_string()
                } else {
                    let mut abs_path = current_node.lock().unwrap().get_path().to_string_lossy().to_string();
                    abs_path.push('/');
                    abs_path.push_str(&new_path);
                    abs_path
                };
            
                // Perform the move
                kernel_guard.move_item(abs_original_path.clone(), abs_new_path.clone());
            }
            
            println!("Moved {} to {}.", original_path, new_path);
        }
        
        Command::Undo(index) => {
            {let mut kernel_guard = kernel.lock().unwrap();
            kernel_guard.undo_deletion(index);}
            println!("Undid deletion of index {}.", index);
        }
        Command::Display => {
            let display = {
                let kernel_guard = kernel.lock().unwrap();
                kernel_guard.display(current_node.clone())
            };
            println!("{}", display);
        }
        Command::Up => {
            let (response, new_node) = {
                let kernel_guard = kernel.lock().unwrap(); // Lock the kernel
                if let Some(parent) = kernel_guard.get_parent(current_node.clone()) {
                    if let Some(upgraded) = parent.upgrade() {
                        ("Moved up to parent directory.".to_string(), Some(upgraded)) // Success case
                    } else {
                        ("Error upgrading parent reference.".to_string(), None) // Failed to upgrade
                    }
                } else {
                    ("Already at the root directory.".to_string(), None) // No parent exists
                }
            }; // End of the scope for the lock
        
            if let Some(new_node) = new_node {
                println!("{}", response);
                return new_node; // Return the updated parent node
            } else {
                println!("{}", response);
            }
        
        }
        Command::Down(index) => {
            let (response, child) = {
                let kernel_guard = kernel.lock().unwrap();
                if let Some(child_node) = kernel_guard.get_child(current_node.clone(), index) {
                    ("Moved down to child directory.".to_string(), Some(child_node))
                } else {
                    ("Invalid child index.".to_string(), None)
                }
            };

            if let Some(new_node) = child {
                println!("{}", response);
                return new_node; // Return the updated parent node
            } else {
                println!("{}", response);
            }
        }
        Command::Commit => {
            {let mut kernel_guard = kernel.lock().unwrap();
            kernel_guard.commit_actions();}
            println!("Committed all actions.");
        }
        Command::Status => {
            let status = {
                let kernel_guard = kernel.lock().unwrap();
                kernel_guard.get_status()
            };
            println!("{}", status);
        }
        Command::GoTo(path) => {
            let (response, node) = {
                let kernel_guard = kernel.lock().unwrap();
                if let Some(node) = kernel_guard.go_to(path.clone()) {
                    (format!("Navigated to {}.", path), Some(node))
                } else {
                    (format!("Invalid path: {}.", path), None)
                }
            };

            if let Some(new_node) = node {
                println!("{}", response);
                return new_node;
            } else {
                println!("{}", response);
            }

        }
        Command::Open(index) => {
            {let kernel_guard = kernel.lock().unwrap();
            kernel_guard.open_file(current_node.clone(), index);}
            println!("Opened file at index {}.", index);
        }
        Command::Help => {
            let help_message = r#"
        Available Commands:
        1. `..` - Moves up one level.
        2. `<index>` - Moves down to the child at the specified index.
        3. `go to <path>` - Navigates to the specified path.
        4. `commit` - Commits the current state.
        5. `undo <index>` - Reverts to a specific commit index.
        6. `status` - Displays the current status.
        7. `display` - Displays content or structure at the current level.
        8. `create file <name>` - Creates a file with the specified name.
        9. `create folder <name>` - Creates a folder with the specified name.
        10. `del <index>` - Deletes the item at the specified index.
        11. `open <index>` - Opens the item at the specified index.
        12. `move <source> > <destination>` - Moves an item from source to destination.
        13. `help` - Displays this help message.
        "#;
            println!("{}", help_message.to_string());
        }
        Command::AISuggestion(input) => {
            let context = {
                let kernel_guard = kernel.lock().unwrap();
                kernel_guard.display(current_node.clone())
            };
            let response = ai::ask(input, context).await;
            
            let suggestion: String = {
                if let Some(sug) = response {
                    kernel.lock().unwrap().set_suggestion(sug.clone());
                    sug
                } else {
                    "No Suggestions".to_string()
                }
            };

            println!("{}", suggestion);

        }
        Command::AIConfirm => {
            {kernel.lock().unwrap().convert_suggestions(current_node.clone());}

            println!("Applied suggestions (Still needs to be committed)");
        }
        Command::Exit => {
            println!("Exiting backend.");
        }
        _ => {
            println!("Unknown command.");
        }
    }

    // Return the current node if no change
    current_node
}
