mod config;
mod messages;

use std::env;
use std::fs;
use std::collections::HashSet;
use std::io::{self, Write};
use std::io::{Error, ErrorKind};
use std::process::{Command, Stdio};

use crate::config::{Action, ActionTypeSpec, SendMailConfig, ServerHealthConfig, Target};
use crate::messages::MailMessage;

use reqwest::{Client, Response};
//use serde::{Deserialize, Serialize};
use tokio::time::{self, Duration};

use mail_builder::MessageBuilder;
use mail_send::{Credentials, SmtpClientBuilder};

/*
 * TODO: inotify to wait on config: https://docs.rs/inotify/latest/inotify/struct.Inotify.html#method.read_events
 */

#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub status_code: u16,
    pub overall_ok: bool,
    pub exec_error_msg: String
}

async fn update_config_indicate_success(
    config_path: String,
    previous_config_opt: &Option<ServerHealthConfig>,
) -> (ServerHealthConfig, bool) {
    let contents =
        fs::read_to_string(config_path.clone()).expect("Should have been able to read the file");

    let parse_result = serde_json::from_str(&contents);
    match parse_result {
        Ok(config) => (config, true),
        Err(e) => {
            if previous_config_opt.is_some() {
                let msg = messages::MailMessage {
                    body: format!("server health failed to parse config file {} due to error: {}", config_path, e),
                    subject: "server health failed to parse config file".to_string(),
                };
                send_mail(&previous_config_opt.as_ref().unwrap().send_mail, &msg, build_mail_recipients(&previous_config_opt.as_ref().unwrap().admin_recipients).as_ref()).await;
                return (previous_config_opt.as_ref().unwrap().clone(), false);
            }
            else {
                panic!("could not load initial config, aborting")
            }
        }
    }
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
    let mut previous_config_opt: Option<ServerHealthConfig> = None;
    let mut targets_known_down: HashSet<usize> = HashSet::new();
    loop {
        // re-read the config after each check
        println!("going to read the config file");
        let (new_config, read_config_success) = update_config_indicate_success(input_file_path.clone(), &previous_config_opt).await;
        previous_config_opt = Some(new_config.clone());
        if read_config_success  {
            for (t_idx, target) in new_config.targets.iter().enumerate() {
                let is_up_now = process_target_report_is_up(&target, &new_config.send_mail, !targets_known_down.contains(&t_idx)).await;
                if !is_up_now {
                    targets_known_down.insert(t_idx);
                }
                else
                {
                  targets_known_down.remove(&t_idx);
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(new_config.watch_intervall_secs)).await;

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

async fn check_website_health(url: &String) -> Result<ServerStatus, reqwest::Error> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let response = client.get(url).send().await?; 

    let status_code = response.status().as_u16();
    println!("response = {}", status_code);
    if status_code != 200 {
        println! {"unexpected status code from server!"}
        return Ok(ServerStatus {
            status_code,
            overall_ok: false,
            exec_error_msg: String::new()
        });
    } else {
        return Ok(ServerStatus {
            status_code,
            overall_ok: true,
            exec_error_msg: String::new()
        });
    }
}

async fn send_mail(
    send_mail_config: &SendMailConfig,
    msg: &MailMessage,
    recipients: &Vec<(String, String)>,
) {
    println!("sending mail to {} recipients", recipients.len());
    // Build a simple multipart message
    let message = MessageBuilder::new()
        .from((
            send_mail_config.mail_address.clone(),
            send_mail_config.mail_address.clone(),
        ))
        //.to(vec![ ("Jane Doe", "falkostrenzke@gmail.com"), //,
        .to(recipients.to_owned())
        .subject(msg.subject.clone())
        .html_body(format!("<p>{}</p>", msg.body))
        .text_body(format!("{}", msg.body));

    // Connect to the SMTP submissions port, upgrade to TLS and
    // authenticate using the provided credentials.
    //let cred : Credentials<&String> = Credentials::Plain {username: &"".to_string(), secret: &"".to_string()};
    let cred: Credentials<&String> =
        Credentials::new(&send_mail_config.user_name, &send_mail_config.password);
    SmtpClientBuilder::new(&send_mail_config.smtp_url, send_mail_config.port)
        .credentials(cred)
        .connect()
        .await
        .unwrap()
        .send(message)
        .await
        .unwrap();
}

async fn server_health_retries(target_conf: &Target, nb_tries: u16) -> ServerStatus {
    let mut result = ServerStatus {
        status_code: 0,
        overall_ok: false,
        exec_error_msg: String::new()
    };
    for _ in 0..nb_tries {
        let server_status_result = check_website_health(&target_conf.watch_url);
        match server_status_result.await {
            Ok(status) => {
                result = status.clone();
                if result.overall_ok {
                    break;
                }
            }
            ,
            Err(e) => {
    result = ServerStatus {
        status_code: 0,
        overall_ok: false,
        exec_error_msg: format!("Exception occured: {}", e),
    };
            }
        }
        // Note: currently, all results except the one from the last retry get swallowed here and
        // are not reported.
        tokio::time::sleep(Duration::from_secs(target_conf.wait_between_tries_secs)).await;
    }
    return result;
}

fn build_mail_recipients(vec_of_mail_address: &Vec<String>) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = Vec::new();
    for r in vec_of_mail_address {
        result.push((r.clone(), r.clone()));
    }
    return result;
}

async fn process_target_report_is_up(target_conf: &Target, send_mail_config: &SendMailConfig, target_known_up: bool) -> bool {
    let nb_tries = target_conf.retries_before_actions + 1;
    let mut server_status = server_health_retries(&target_conf, nb_tries).await;
    let recipients = build_mail_recipients(&target_conf.recipients);
    let mut last_action_output: Option<String> = None;
    //let actions_exhausted = false;
    let mut action_idx: usize = 0;
    if !server_status.overall_ok {
        while action_idx < target_conf.actions.len() {
            if target_known_up {
                let msg: MailMessage = messages::make_message_will_take_action(
                    &target_conf,
                    action_idx,
                    &server_status,
                    &last_action_output,
                );
                send_mail(send_mail_config, &msg, &recipients).await;
            }
            let action: &Action = &target_conf.actions[action_idx];
            for _ in 0..action.repeat_times {
                last_action_output = Some(run_spefice_action(action));
                tokio::time::sleep(Duration::from_secs(action.wait_afterwards_secs)).await;
                server_status = server_health_retries(&target_conf, nb_tries).await;
                if server_status.overall_ok {
                    break;
                }
            }
            action_idx += 1;
            if action_idx == target_conf.actions.len() {
                break;
            }
            if server_status.overall_ok {
                break;
            }
        }
    }
    if !server_status.overall_ok {
        if target_known_up {
            let msg = messages::make_message_actions_exhausted(target_conf, &server_status);
            send_mail(send_mail_config, &msg, &recipients).await;
        }
    }
    else {
        if !target_known_up {
            let msg: MailMessage = messages::make_message_target_up_again(
                &target_conf,
            );
            send_mail(send_mail_config, &msg, &recipients).await;
        }
        return true;
    }
    return server_status.overall_ok;
}

fn run_spefice_action(action: &Action) -> String {
    match &action.typespecific {
        ActionTypeSpec::RunScript { path_to_script } => {
            let script_result = run_script(path_to_script.as_str());
            match script_result {
                Ok(text) => {
                    format!("script output: {}", text)
                }
                Err(e) => {
                    format!("error when running script: {}", e)
                }
            }
        }
    }
}
