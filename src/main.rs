
mod config;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::io::{Error, ErrorKind};
use std::process::{Command, Stdio};

use crate::config::{ServerHealthConfig,SendMailConfig, Target, Action};

use reqwest::{Client, Response};
//use serde::{Deserialize, Serialize};
use tokio::time::{self, Duration};
use std::task;

use mail_builder::MessageBuilder;
use mail_send::{SmtpClientBuilder,Credentials};


/*
 * send mails: https://docs.rs/mail-send/latest/mail_send/
 *
 */

#[derive(Debug, Clone)]
pub struct ServerStatus {
pub status_code: u16,
pub overall_ok: bool
}

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

    for target in conf.targets {
        process_target(&target, &send_mail_config).await;
    }

    // let resp = check_websites();
    // let status_code = resp.await.status().as_u16();
    // send_mail(send_mail_config).await;
    let script_result = run_script("/usr/bin/ls");
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

async fn check_website_health(url : &String) -> ServerStatus {
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
        .get(url)
        .send()
        .await
        .unwrap(); // catch dns lookup error

    let status_code = response.status().as_u16();
    println!("response = {}", status_code);
    if status_code != 200{
        println! {"unexpected status code from server!"}
        return ServerStatus{ status_code, overall_ok: false};
    } else {
        return ServerStatus{ status_code, overall_ok: true};
    }
}

async fn send_mail(send_mail_config : &SendMailConfig, body: &String, recipients: &Vec<(String, String)>) {
    println!("sending mail to {} recipients", recipients.len());
   // recipients : Vec<(String, String)> = Vec::new();
    // Build a simple multipart message
    let message = MessageBuilder::new()
        .from((send_mail_config.mail_address.clone(), send_mail_config.mail_address.clone()))
        //.to(vec![ ("Jane Doe", "falkostrenzke@gmail.com"), //,
        .to(recipients.to_owned())
        .subject("Hi!")
        .html_body(format!("<h1>{}</h1>", body))
        .text_body(format!("{}", body));

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

async fn server_health_retries(target_conf : &Target, nb_tries : u16) -> ServerStatus {
    let mut result = ServerStatus { status_code: 0, overall_ok: false};
    for _ in 0..nb_tries {
       let server_status = check_website_health(&target_conf.watch_url) ;
        result = server_status.await.clone();
       if result.overall_ok {
          break;   
       }
        tokio::time::sleep(Duration::from_secs(target_conf.wait_between_tries_secs)).await;
        //task::sleep(delay).await;
    }
    return result;
}

fn build_mail_recipients(target_conf : &Target) -> Vec<(String, String)> {
    let mut result : Vec<(String, String)> = Vec::new();
    for r in &target_conf.recipients {
        result.push((r.clone(), r.clone()));
    }
    return result;
}

async fn process_target(target_conf: &Target, send_mail_config: &SendMailConfig) {
    let nb_tries = target_conf.retries_before_actions + 1;
    let server_status = server_health_retries(&target_conf, nb_tries).await;
    let recipients = build_mail_recipients(target_conf);
    if !server_status.overall_ok  {
        let msg : String = format!("server status of target {} not OK (status code = {})", target_conf.watch_url, server_status.status_code);
        send_mail(send_mail_config, &msg, &recipients).await;
    }
    // TODO: execute actions, check health after each and inform recipients
}
