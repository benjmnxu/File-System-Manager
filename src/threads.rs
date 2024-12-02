use tokio::sync::mpsc;

pub enum Command {
    LoadDirectory(String),
    Del(usize),
    Move(String, String),
    Create(String, bool),
    Undo(usize),
    Commit,
    Display,
    Up,
    Down(usize),
    Status,
    Open(usize),
    GoTo(String),
    Exit,
    Error(String),
}

pub enum BackendResponse {
    Response(String),
    Error(String),
}

pub async fn send_response(to_gui: &mpsc::Sender<BackendResponse>, message: String) {
    if let Err(err) = to_gui.send(BackendResponse::Response(message)).await {
        eprintln!("Failed to send response to GUI: {}", err);
    }
}

pub async fn send_error(to_gui: &mpsc::Sender<BackendResponse>, error: String) {
    if let Err(err) = to_gui.send(BackendResponse::Error(error)).await {
        eprintln!("Failed to send error to GUI: {}", err);
    }
}

pub async fn send_command(to_gui: &mpsc::Sender<BackendResponse>, message: String) {
    if let Err(err) = to_gui.send(BackendResponse::Response(message)).await {
        eprintln!("Failed to send response to GUI: {}", err);
    }
}

pub fn read_message(from_gui: &mpsc::Receiver<Command>) {

}

pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    if input == ".." {
        Command::Up
    } else if input == "commit" {
        Command::Commit
    } else if input.starts_with("del ") {
        if let Ok(index) = input[4..].trim().parse::<usize>() {
            Command::Del(index)
        } else {
            Command::Error("Invalid command".to_string())
        }
    } else if input.starts_with("undo ") {
        if let Ok(index) = input[5..].trim().parse::<usize>() {
            Command::Undo(index)
        } else {
            Command::Error("Invalid command".to_string())
        }
    } else if input == "status" {
        Command::Status
    } else if input == "display" {
        Command::Display
    } else if let Ok(index) = input.parse::<usize>() {
        Command::Down(index)
    } else if input.starts_with("open ") {
        if let Ok(index) = input[5..].trim().parse::<usize>() {
            Command::Open(index)
        } else {
            Command::Error("Invalid command".to_string())
        }
    } else if input.starts_with("go to ") {
        if let Ok(path) = input[6..].trim().parse::<String>() {
            Command::GoTo(path)
        } else {
            Command::Error("Invalid command".to_string())
        }
    } else if input.starts_with("create ") {
        if let Ok(item_type) = input[7..].trim().parse::<String>() {
            if item_type.starts_with("file ") {
                if let Ok(file_name) = item_type[5..].trim().parse::<String>() {
                    Command::Create(file_name, true)
                } else {
                    Command::Error("Invalid command".to_string())
                }
            } else if item_type.starts_with("folder ") {
                if let Ok(file_name) = item_type[7..].trim().parse::<String>() {
                    Command::Create(file_name, false)
                } else {
                    Command::Error("Invalid command".to_string())
                }
            } else {
                Command::Error("Invalid command".to_string())
            }
        } else {
            Command::Error("Invalid command".to_string())
        }
    } else if input.starts_with("move ") {
        if let Ok(paths) = input[5..].trim().parse::<String>() {
            let paths_vec: Vec<&str> = paths.split(">").collect();
            if paths_vec.len() != 2 {
                Command::Error("Invalid command".to_string())
            } else {
                Command::Move(paths_vec[0].to_string(), paths_vec[1].to_string())
            }
        } else {
            Command::Error("Invalid command".to_string())
        }
    } else {
        Command::Error("Invalid command".to_string())
    }
}