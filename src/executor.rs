use crate::error::{AppError, Result};
use crate::models::{FileInput, Node, Param, ParamValue, ParamValues};
#[cfg(not(feature = "fire-and-forget"))]
use futures_util::StreamExt;
use reqwest::{multipart, Client, Method};
use shlex;
use std::path::Path;
use std::process::Stdio;
#[cfg(not(feature = "fire-and-forget"))]
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
#[cfg(feature = "fire-and-forget")]
use tokio_util::io::ReaderStream;

pub enum ExecutionPlan {
    Http {
        method: Method,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
        multipart_fields: Vec<MultipartField>,
    },
    Command {
        program: String,
        args: Vec<String>,
    },
    NoExecution {
        function_name: String,
        parameters: ParamValues,
    },
}

pub enum MultipartField {
    Text {
        name: String,
        value: String,
    },
    File {
        name: String,
        file: FileInput,
    },
}

pub struct RawExecutionOutput {
    pub bytes: Vec<u8>,
}

impl RawExecutionOutput {
    #[cfg(feature = "fire-and-forget")]
    pub fn empty() -> Self {
        Self { bytes: Vec::new() }
    }
}

fn replace_optional_placeholder(rendered: &mut String, name: &str) {
    let quoted_placeholder = format!("\"#{}\"", name);
    let placeholder = format!("#{}", name);
    *rendered = rendered.replace(&quoted_placeholder, "null");
    *rendered = rendered.replace(&placeholder, "");
}

pub fn render_template(
    template: &str,
    params: &ParamValues,
    func_params: &[Param],
) -> String {
    let mut rendered = template.to_string();

    for func_param in func_params {
        let placeholder = format!("#{}", func_param.name);
        match params.get(&func_param.name) {
            Some(ParamValue::Text(value)) => {
                rendered = rendered.replace(&placeholder, value);
            }
            Some(ParamValue::File(file)) => {
                rendered = rendered.replace(&placeholder, &file.path);
            }
            Some(ParamValue::Files(files)) => {
                let joined = files
                    .iter()
                    .map(|file| file.path.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                rendered = rendered.replace(&placeholder, &joined);
            }
            None if !func_param.required => {
                replace_optional_placeholder(&mut rendered, &func_param.name);
            }
            None => {
                rendered = rendered.replace(&placeholder, "");
            }
        }
    }

    rendered
}

fn parse_form_field(token: &str, params: &ParamValues) -> Result<Vec<MultipartField>> {
    let (name, raw_value) = token.split_once('=').ok_or_else(|| {
        AppError::BadRequest(format!("Invalid multipart field: {}", token))
    })?;

    if let Some(param_name) = raw_value.strip_prefix("@#") {
        return match params.get(param_name) {
            Some(ParamValue::File(file)) => Ok(vec![MultipartField::File {
                name: name.to_string(),
                file: file.clone(),
            }]),
            Some(ParamValue::Files(files)) => Ok(files
                .iter()
                .cloned()
                .map(|file| MultipartField::File {
                    name: name.to_string(),
                    file,
                })
                .collect()),
            Some(ParamValue::Text(_)) => Err(AppError::BadRequest(format!(
                "Parameter '{}' is text, but curl uses it as a file",
                param_name
            ))),
            None => Ok(Vec::new()),
        };
    }

    if let Some(path) = raw_value.strip_prefix('@') {
        return Ok(vec![MultipartField::File {
            name: name.to_string(),
            file: FileInput {
                path: path.to_string(),
                filename: None,
                content_type: None,
                label: None,
            },
        }]);
    }

    Ok(vec![MultipartField::Text {
        name: name.to_string(),
        value: raw_value.to_string(),
    }])
}

fn parse_header(value: &str) -> Option<(String, String)> {
    let (name, value) = value.split_once(':')?;
    Some((name.to_string(), value.trim().to_string()))
}

fn parse_json_body(value: &str, func_params: &[Param], params: &ParamValues) -> Result<String> {
    if let Ok(mut json_val) = serde_json::from_str::<serde_json::Value>(value) {
        if let Some(obj) = json_val.as_object_mut() {
            let mut keys_to_remove = Vec::new();
            for (key, value) in obj.iter() {
                if value.is_null() {
                    if let Some(param) = func_params.iter().find(|param| param.name == *key) {
                        if !param.required && params.get(&param.name).is_none() {
                            keys_to_remove.push(key.clone());
                        }
                    }
                }
            }
            for key in keys_to_remove {
                obj.remove(&key);
            }
        }

        return serde_json::to_string(&json_val).map_err(|e| {
            AppError::InternalServerError(format!("Failed to serialize JSON: {}", e))
        });
    }

    Ok(value.to_string())
}

fn expand_command_arg(arg: &str, params: &ParamValues) -> Vec<String> {
    let Some(name) = arg.strip_prefix('#') else {
        return vec![arg.to_string()];
    };

    match params.get(name) {
        Some(ParamValue::Text(value)) => vec![value.clone()],
        Some(ParamValue::File(file)) => vec![file.path.clone()],
        Some(ParamValue::Files(files)) => files.iter().map(|file| file.path.clone()).collect(),
        None => Vec::new(),
    }
}

pub fn parse_curl(
    template: &str,
    params: &ParamValues,
    func_params: &[Param],
) -> Result<(Method, String, Vec<(String, String)>, Option<String>, Vec<MultipartField>)> {
    let rendered_template = render_template(template, params, func_params);
    let parts = shlex::split(&rendered_template)
        .ok_or_else(|| AppError::BadRequest("Failed to parse curl template".to_string()))?;

    let mut method = Method::POST;
    let mut url = None;
    let mut headers = Vec::new();
    let mut body = None;
    let mut multipart_fields = Vec::new();

    let mut i = 0;
    while i < parts.len() {
        match parts[i].as_str() {
            "curl" => {
                i += 1;
            }
            "-X" => {
                let value = parts.get(i + 1).ok_or_else(|| {
                    AppError::BadRequest("Missing value after -X".to_string())
                })?;
                method = value.parse().map_err(|_| {
                    AppError::BadRequest(format!("Invalid HTTP method: {}", value))
                })?;
                i += 2;
            }
            "-H" => {
                let value = parts.get(i + 1).ok_or_else(|| {
                    AppError::BadRequest("Missing value after -H".to_string())
                })?;
                if let Some(header) = parse_header(value) {
                    headers.push(header);
                }
                i += 2;
            }
            "-d" | "--data" | "--data-raw" => {
                let value = parts.get(i + 1).ok_or_else(|| {
                    AppError::BadRequest(format!("Missing value after {}", parts[i]))
                })?;
                body = Some(parse_json_body(value, func_params, params)?);
                i += 2;
            }
            "-F" | "--form" => {
                let value = parts.get(i + 1).ok_or_else(|| {
                    AppError::BadRequest(format!("Missing value after {}", parts[i]))
                })?;
                multipart_fields.extend(parse_form_field(value, params)?);
                i += 2;
            }
            _ => {
                if parts[i].starts_with("http") {
                    url = Some(parts[i].clone());
                }
                i += 1;
            }
        }
    }

    let url = url.ok_or_else(|| AppError::BadRequest("URL not found in curl template".to_string()))?;

    if body.is_some() && !multipart_fields.is_empty() {
        return Err(AppError::BadRequest(
            "curl template cannot mix -d and -F in the current executor".to_string(),
        ));
    }

    Ok((method, url, headers, body, multipart_fields))
}

pub fn build_execution_plan(node: &Node, params: &ParamValues) -> Result<ExecutionPlan> {
    let execution_type = node
        .execution_type
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| {
            if node.command_program.is_some() || node.command_template.is_some() {
                "command".to_string()
            } else {
                "http".to_string()
            }
        });

    match execution_type.as_str() {
        "http" => {
            let template = if let Some(template) = node.request_template.as_deref() {
                template
            } else {
                return Ok(ExecutionPlan::NoExecution {
                    function_name: node.name.clone(),
                    parameters: params.clone(),
                });
            };
            let (method, url, headers, body, multipart_fields) =
                parse_curl(template, params, &node.parameters)?;
            Ok(ExecutionPlan::Http {
                method,
                url,
                headers,
                body,
                multipart_fields,
            })
        }
        "command" | "local" | "cli" => {
            if let Some(program) = node.command_program.as_deref() {
                let rendered_program = render_template(program, params, &node.parameters);
                let mut args = Vec::new();

                for arg in &node.command_args {
                    if arg.starts_with('#') {
                        args.extend(expand_command_arg(arg, params));
                    } else {
                        args.push(render_template(arg, params, &node.parameters));
                    }
                }

                return Ok(ExecutionPlan::Command {
                    program: rendered_program,
                    args,
                });
            }

            let template = if let Some(template) = node.command_template.as_deref() {
                template
            } else {
                return Ok(ExecutionPlan::NoExecution {
                    function_name: node.name.clone(),
                    parameters: params.clone(),
                });
            };
            let rendered = render_template(template, params, &node.parameters);
            let parts = shlex::split(&rendered).ok_or_else(|| {
                AppError::BadRequest("Failed to parse command template".to_string())
            })?;
            let (program, args) = parts.split_first().ok_or_else(|| {
                AppError::BadRequest("Command template is empty".to_string())
            })?;
            Ok(ExecutionPlan::Command {
                program: program.clone(),
                args: args.to_vec(),
            })
        }
        other => Err(AppError::BadRequest(format!(
            "Unsupported execution type: {}",
            other
        ))),
    }
}

pub async fn execute_http(
    client: &Client,
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    multipart_fields: Vec<MultipartField>,
) -> Result<RawExecutionOutput> {
    #[cfg(feature = "fire-and-forget")]
    {
        let client = client.clone();
        tokio::spawn(async move {
            if let Err(err) = execute_http_detached(&client, method, url, headers, body, multipart_fields).await {
                eprintln!("Detached HTTP execution failed: {}", err);
            }
        });
        return Ok(RawExecutionOutput::empty());
    }

    #[cfg(not(feature = "fire-and-forget"))]
    {
        execute_http_blocking(client, method, url, headers, body, multipart_fields).await
    }
}

#[cfg(feature = "fire-and-forget")]
async fn execute_http_detached(
    client: &Client,
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    multipart_fields: Vec<MultipartField>,
) -> Result<()> {
    let mut req = client.request(method, &url);

    for (k, v) in headers {
        req = req.header(k, v);
    }

    if !multipart_fields.is_empty() {
        req = req.multipart(build_multipart_form(multipart_fields).await?);
    } else if let Some(body) = body {
        req = req.body(body);
    }

    let response = req.send().await?;
    println!("Detached HTTP Response Status: {}", response.status());
    Ok(())
}

#[cfg(not(feature = "fire-and-forget"))]
async fn execute_http_blocking(
    client: &Client,
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    multipart_fields: Vec<MultipartField>,
) -> Result<RawExecutionOutput> {
    let mut req = client.request(method, &url);

    for (k, v) in headers {
        req = req.header(k, v);
    }

    if !multipart_fields.is_empty() {
        req = req.multipart(build_multipart_form(multipart_fields).await?);
    } else if let Some(body) = body {
        req = req.body(body);
    }

    let response = req.send().await?;
    let status = response.status();
    let mut stream = response.bytes_stream();
    let mut response_body = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|e| AppError::InternalServerError(format!("Failed to read response body: {}", e)))?;
        response_body.extend_from_slice(&chunk);
    }

    println!("HTTP Response Status: {}", status);
    println!("HTTP Response Body:\n{}", String::from_utf8_lossy(&response_body));

    Ok(RawExecutionOutput {
        bytes: response_body,
    })
}

async fn build_multipart_form(multipart_fields: Vec<MultipartField>) -> Result<multipart::Form> {
    let mut form = multipart::Form::new();
    for field in multipart_fields {
        match field {
            MultipartField::Text { name, value } => {
                form = form.text(name, value);
            }
            MultipartField::File { name, file } => {
                form = form.part(name, build_file_part(file).await?);
            }
        }
    }
    Ok(form)
}

async fn build_file_part(file: FileInput) -> Result<multipart::Part> {
    let file_path = file.path.clone();
    let fallback_filename = Path::new(&file_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("upload.bin")
        .to_string();
    let filename = file.filename.unwrap_or(fallback_filename);

    #[cfg(feature = "fire-and-forget")]
    let mut part = {
        let metadata = tokio::fs::metadata(&file_path).await.map_err(|e| {
            AppError::BadRequest(format!("Failed to stat file '{}': {}", file_path, e))
        })?;
        let source = tokio::fs::File::open(&file_path).await.map_err(|e| {
            AppError::BadRequest(format!("Failed to open file '{}': {}", file_path, e))
        })?;
        let stream = ReaderStream::new(source);
        multipart::Part::stream_with_length(reqwest::Body::wrap_stream(stream), metadata.len())
            .file_name(filename)
    };

    #[cfg(not(feature = "fire-and-forget"))]
    let mut part = {
        let bytes = tokio::fs::read(&file_path).await.map_err(|e| {
            AppError::BadRequest(format!("Failed to read file '{}': {}", file_path, e))
        })?;
        multipart::Part::bytes(bytes).file_name(filename)
    };

    if let Some(content_type) = file.content_type {
        part = part.mime_str(&content_type).map_err(|e| {
            AppError::BadRequest(format!(
                "Invalid content type for file '{}': {}",
                file_path, e
            ))
        })?;
    }

    Ok(part)
}

pub async fn execute_command(program: String, args: Vec<String>) -> Result<RawExecutionOutput> {
    #[cfg(feature = "fire-and-forget")]
    {
        let detached_program = program.clone();
        tokio::spawn(async move {
            if let Err(err) = execute_command_detached(detached_program, args).await {
                eprintln!("Detached command execution failed: {}", err);
            }
        });
        return Ok(RawExecutionOutput::empty());
    }

    #[cfg(not(feature = "fire-and-forget"))]
    {
        execute_command_blocking(program, args).await
    }
}

#[cfg(feature = "fire-and-forget")]
async fn execute_command_detached(program: String, args: Vec<String>) -> Result<()> {
    Command::new(&program)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| AppError::InternalServerError(format!("Failed to execute command '{}': {}", program, e)))?;
    Ok(())
}

#[cfg(not(feature = "fire-and-forget"))]
async fn read_stream<R>(reader: Option<R>) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let Some(mut reader) = reader else {
        return Ok(Vec::new());
    };

    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to read process stream: {}", e)))?;
    Ok(buffer)
}

#[cfg(not(feature = "fire-and-forget"))]
async fn execute_command_blocking(program: String, args: Vec<String>) -> Result<RawExecutionOutput> {
    let mut child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::InternalServerError(format!("Failed to execute command '{}': {}", program, e)))?;

    let stdout_handle = tokio::spawn(read_stream(child.stdout.take()));
    let stderr_handle = tokio::spawn(read_stream(child.stderr.take()));
    let status = child
        .wait()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to wait for command '{}': {}", program, e)))?;

    let stdout = stdout_handle
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to join stdout task: {}", e)))??;
    let stderr = stderr_handle
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to join stderr task: {}", e)))??;

    let stdout_text = String::from_utf8_lossy(&stdout);
    let stderr_text = String::from_utf8_lossy(&stderr);

    if !stdout_text.is_empty() {
        println!("Command stdout:\n{}", stdout_text);
    }
    if !stderr_text.is_empty() {
        eprintln!("Command stderr:\n{}", stderr_text);
    }

    if status.success() {
        if stdout.is_empty() {
            let mut message = format!("Command '{}' completed successfully", program);
            if !stderr_text.is_empty() {
                message.push_str(&format!("\nstderr:\n{}", stderr_text));
            }
            Ok(RawExecutionOutput {
                bytes: message.into_bytes(),
            })
        } else {
            let mut output = stdout;
            if !stderr_text.is_empty() {
                output.extend_from_slice(b"\nstderr:\n");
                output.extend_from_slice(stderr_text.as_bytes());
            }
            Ok(RawExecutionOutput { bytes: output })
        }
    } else {
        let status = status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "terminated by signal".to_string());
        let mut message = format!("Command '{}' failed with status {}", program, status);
        if !stdout_text.is_empty() {
            message.push_str(&format!("\nstdout:\n{}", stdout_text));
        }
        if !stderr_text.is_empty() {
            message.push_str(&format!("\nstderr:\n{}", stderr_text));
        }
        eprintln!("Error: {}", message);
        Err(AppError::InternalServerError(message))
    }
}
