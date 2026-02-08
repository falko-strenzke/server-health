#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerHealthConfig {
    pub send_mail: SendMailConfig,
    pub watch_intervall_secs: u64,
    pub admin_recipients: Vec<String>,
    pub targets : Vec<Target>
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SendMailConfig {
    pub mail_address: String,
    pub user_name: String,
    pub smtp_url: String,
    pub password: String,
    pub port: u16,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Target {
    pub informative_name : String,
    pub watch_url: String,
    pub timeout_secs: u16,
    pub retries_before_actions: u16,
    pub wait_between_tries_secs: u64,
    pub recipients: Vec<String>,
    pub actions : Vec<Action>
}



#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Action {
    pub informative_name : String,
    pub wait_afterwards_secs : u64, 
    pub repeat_times: u16,
    pub typespecific : ActionTypeSpec
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ActionTypeSpec {
    RunScript { path_to_script : String  }
    }

