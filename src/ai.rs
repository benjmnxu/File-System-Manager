use async_openai::{
    types::{
        ChatCompletionFunctionsArgs, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, Role
    },
    Client,
};
use serde_json::json;
use serde::Deserialize;

pub async fn ask(question: String, context: String) -> Option<String> {
    // Initialize OpenAI client
    println!("Using API Key: {:?}", std::env::var("OPENAI_API_KEY"));
    let client = Client::new();
    let model = "gpt-4o-mini";

    // Define GPT system prompt for modifying the filesystem
    let system_prompt = r#"
    You are an assistant who specializes in cleaning up and organizing file systems using three commands: Delete, Move, and Create.
    You will be given a filesystem on which to work. It is your responsibility to clean up the provided system and propose solutions
    to free up as much space as possible. YOU CAN ONLY USE THE PROVIDED FUNCTION CALLS. Create a list of function calls as your answer.
    EVERY ENTRY IN YOUR LIST SHOULD BE IN JSON FORMAT. ONLY RETURN THE LIST AND NOTHING ELSE. DO NOT HAVE ANY ADDITIONAL CHARACTERS.
    
    For Example: 
    [
        {"delete_file": { "path": "/Users/benjaminxu/Desktop/10.9783_9780812295061-toc.pdf"}},
        {"delete_file": { "path": "/Users/benjaminxu/Desktop/homework_3_written.pdf"}},
        {"move_item": {"original_location": "/Users/benjaminxu/Desktop/situation 1.m4a", "new_location": "/Users/benjaminxu/Desktop/Segggsss/situation 1.m4a"}}
    ]
    "#;

    let user_prompt = format!("{} \n {}", question, context);

    // Prepare the GPT request
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages([
            ChatCompletionRequestMessageArgs::default()
                .content(system_prompt)
                .role(Role::System)
                .build()
                .ok()?,
            ChatCompletionRequestMessageArgs::default()
                .content(user_prompt)
                .role(Role::User)
                .build()
                .ok()?,
        ])
        .functions([
            ChatCompletionFunctionsArgs::default()
                .name("create_directory")
                .description("Create a directory at the specified path")
                .parameters(json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The directory path to create, e.g., /home/user/new_folder",
                        },
                    },
                    "required": ["path"],
                }))
                .build()
                .ok()?,
            ChatCompletionFunctionsArgs::default()
                .name("delete_file")
                .description("Delete a file at the specified path")
                .parameters(json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to delete, e.g., /home/user/file.txt",
                        },
                    },
                    "required": ["path"],
                }))
                .build()
                .ok()?,
            ChatCompletionFunctionsArgs::default()
                .name("move_item")
                .description("Move a file or directory to any other location")
                .parameters(json!({
                    "type": "object",
                    "properties": {
                        "original_location": {
                            "type": "string",
                            "description": "The original path",
                        },
                        "new_location": {
                            "type": "string",
                            "description": "The directory path to which the item should be moved",
                        },
                    },
                    "required": ["original_location", "new_location"],
                }))
                .build()
                .ok()?,
        ])
        .function_call("auto")
        .build()
        .ok()?;

    // Send the request to OpenAI
    let response = client.chat().create(request).await.ok()?;
    
    // Extract the response content
    let content = response.choices.get(0)?.message.content.clone();
    
    content
}


#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AICommand {
    DeleteFile { delete_file: DeleteFile },
    MoveItem { move_item: MoveItem },
    CreateDirectory {create_directory: CreateDirectory}
}

#[derive(Debug, Deserialize)]
pub struct DeleteFile {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveItem {
   pub original_location: String,
   pub new_location: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDirectory {
   pub path: String,
}

pub fn parse_ai(json_input: &str) -> Vec<AICommand> {
    // Parse JSON into a vector of commands
    let commands: Vec<AICommand> = match serde_json::from_str(json_input) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            return Vec::new();
        }
    };
    commands
}
