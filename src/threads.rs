use std::sync::mpsc;

pub enum Command {
    LoadDirectory(String),
    Del(usize),
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

pub fn send_response(to_gui: &mpsc::Sender<BackendResponse>, message: String) {
    if let Err(err) = to_gui.send(BackendResponse::Response(message)) {
        eprintln!("Failed to send response to GUI: {}", err);
    }
}

pub fn send_error(to_gui: &mpsc::Sender<BackendResponse>, error: String) {
    if let Err(err) = to_gui.send(BackendResponse::Error(error)) {
        eprintln!("Failed to send error to GUI: {}", err);
    }
}

fn send_command(to_gui: &mpsc::Sender<BackendResponse>, message: String) {
    if let Err(err) = to_gui.send(BackendResponse::Response(message)) {
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
    } else {
        Command::Error("Invalid command".to_string())
    }
}