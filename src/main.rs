
mod config;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::io::{Error, ErrorKind};
use std::process::{Command, Stdio};

use crate::config::{ServerHealthConfig,SendMailConfig};

use reqwest::{Client, Response};
//use serde::{Deserialize, Serialize};
use tokio::time::{self, Duration};

use mail_builder::MessageBuilder;
use mail_send::{SmtpClientBuilder,Credentials};


/*
 * send mails: https://docs.rs/mail-send/latest/mail_send/
 *
 */

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");


    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("must provide json config file path as single argument");
        return;
    }
    let input_file_path = args[1].clone();

    let contents =
        fs::read_to_string(input_file_path).expect("Should have been able to read the file");

    //println!("{contents}")
    //
    //print_example_landscape();

    let conf: ServerHealthConfig = serde_json::from_str(&contents).unwrap();
    let send_mail_config = &conf.send_mail;

    let resp = check_websites();
    let status_code = resp.await.status().as_u16();
    println!("response = {}", status_code);
    if status_code >= 400 {
        println! {"error from server!"}
    } else {
        println! {"server is OK"}
    }
    send_mail(send_mail_config).await;
    let script_result = run_script("/usr/bin/lsalsdfja");
    match script_result {
        Ok(msg) => println!("output from script: {}", msg),
        Err(e) =>  println!("error from script execution: {}", e)
    }
}

pub fn run_script(script_path: &str) -> Result<String, io::Error> {
    let mut child = Command::new(script_path)
        .arg("-la")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let child_stdin = child.stdin.as_mut().unwrap();
    child_stdin.write_all(script_path.as_bytes())?;

    let output_res = child.wait_with_output();
    match output_res {
        Ok(output) => {
            return match str::from_utf8(&output.stdout) {
                Ok(val) => Ok(val.to_string()),
                Err(e) => Err(io::Error::new(
                    ErrorKind::Other,
                    format!("error during UTF-8 decoding of output script: {}", e),
                )),
            }
        }
        Err(e) => {
            return Err(io::Error::new(
                ErrorKind::Other,
                format!("error executing script: {}", e),
            ))
        }
    }
}

async fn check_websites() -> Response {
    // TODO: catch timeout:
    // thread 'main' (2647308) panicked at src/main.rs:28:78:
    // called `Result::unwrap()` on an `Err` value: reqwest::Error { kind: Request, url: "https://dpd
    // ict.net/", source: TimedOut }
    //let client = Client::new();
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        //.timeout(Duration::from_micros(1))
        .build()
        .unwrap();
    let response = client
        .get("https://dpdict.net".to_string())
        .send()
        .await
        .unwrap();
    return response;
}

async fn send_mail(send_mail_config : &SendMailConfig) {
    // Build a simple multipart message
    let message = MessageBuilder::new()
        .from((send_mail_config.mail_address.clone(), send_mail_config.mail_address.clone()))
        .to(vec![
            ("Jane Doe", "falkostrenzke@gmail.com"), //,
                                                     //("James Smith", "james@test.com"),
        ])
        .subject("Hi!")
        .html_body("<h1>Hello, world!</h1>")
        .text_body("Hello world!");

    // Connect to the SMTP submissions port, upgrade to TLS and
    // authenticate using the provided credentials.
    //let cred : Credentials<&String> = Credentials::Plain {username: &"".to_string(), secret: &"".to_string()};
    let cred : Credentials<&String> = Credentials::new(&send_mail_config.user_name, &send_mail_config.password);
    SmtpClientBuilder::new(&send_mail_config.smtp_url, send_mail_config.port)
        .credentials(cred)
        .connect()
        .await
        .unwrap()
        .send(message)
        .await
        .unwrap();
}
