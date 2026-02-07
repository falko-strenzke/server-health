#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerHealthConfig {
    pub send_mail: SendMailConfig,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SendMailConfig {
    pub mail_address: String,
    pub user_name: String,
    pub smtp_url: String,
    pub password: String,
    pub port: u16,
}
