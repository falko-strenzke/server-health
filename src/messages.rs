use crate::{ServerStatus, Target};

pub struct MailMessage {
    pub body: String,
    pub subject: String,
}

pub fn make_message_will_take_action(
    target_conf: &Target,
    action_idx: usize,
    server_status: &ServerStatus,
    last_action_output: &Option<String>
) -> MailMessage {
    let mut result: MailMessage = MailMessage {
        body: "".to_string(),
        subject: "".to_string(),
    };
    let mut last_action_report = String::new();
    if last_action_output.as_ref().is_some_and(|x| x.len() > 0) {
        last_action_report = format!("\nexecution of previous action gave output: {}\n", last_action_output.as_ref().unwrap().clone());
    }
    result.subject = format!("🌐 ⚠️ ⏲️ 🔄 server-health problem report for {}. No action required from you by now.", target_conf.informative_name );
    result. body = format!(
            "server-health's health check found server status of target {} not OK (status code = {}). {}\n server-health will now start action action {} from {} total defined reactions to the outage. You don't have to do anything at this point. server-health will report back after the action's success or failure has been determined. When all actions have been exhausted without success, you will receive a final note.", 
            target_conf.watch_url, server_status.status_code, last_action_report, action_idx + 1, target_conf.actions.len());
    return result;
}


pub fn make_message_actions_exhausted(
    target_conf: &Target,
    server_status: &ServerStatus,
) -> MailMessage {
    let mut result: MailMessage = MailMessage {
        body: "".to_string(),
        subject: "".to_string(),
    };
    result.subject = format!("🌐 ⚠️  ⛔️ server-health STATUS DOWN report for {}. ACTION REQUIRED FROM YOU.", target_conf.informative_name );
    result. body = format!(
            "server-health's health check found server status of target {} not OK (status code = {}). This is the final note informing you that all defined actions have been carried out and the server status is still not healthy.", 
            target_conf.watch_url, server_status.status_code);
    return result;
}

pub fn make_message_target_up_again(target_conf : &Target) -> MailMessage {

    return MailMessage {
        body: format!("server at {} is healthy again.", target_conf.watch_url),
        subject: format!("🌐 💚 🛫 server health report: server {} is up and running again", target_conf.informative_name )
    };
}
