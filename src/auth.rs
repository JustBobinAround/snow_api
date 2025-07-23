use crate::errors::{GlideError, VarOrErr};
#[derive(Debug)]
pub enum CredentialType {
    Basic { user_name: String, passwd: String },
    Token(String),
}

impl VarOrErr for CredentialType {
    fn var_or_err(var_name: &str) -> Result<String, GlideError> {
        match std::env::var(var_name) {
            Ok(var) => Ok(var),
            Err(_) => Err(GlideError::NoApiCredentialsFound),
        }
    }
}

impl CredentialType {
    pub fn load_env_credentials() -> Result<CredentialType, GlideError> {
        let token = CredentialType::var_or_err("SNOW_API_TOKEN");

        match token {
            Ok(token) => Ok(CredentialType::Token(token)),
            Err(_) => {
                let user_name = CredentialType::var_or_err("SNOW_API_USER")?;
                let passwd = CredentialType::var_or_err("SNOW_API_PASSWD")?;
                Ok(CredentialType::Basic { user_name, passwd })
            }
        }
    }
}

#[derive(Debug)]
pub struct GlideRecordConfig {
    credentials: CredentialType,
    snow_instance: String,
}

impl VarOrErr for GlideRecordConfig {
    fn var_or_err(var_name: &str) -> Result<String, GlideError> {
        match std::env::var(var_name) {
            Ok(var) => Ok(var),
            Err(_) => Err(GlideError::NoSnowInstanceFound),
        }
    }
}

impl GlideRecordConfig {
    pub fn new_from_env_vars() -> Result<GlideRecordConfig, GlideError> {
        let credentials = CredentialType::load_env_credentials()?;
        let snow_instance = GlideRecordConfig::var_or_err("SNOW_API_INSTANCE")?;
        Ok(GlideRecordConfig {
            credentials,
            snow_instance,
        })
    }

    pub fn new_with_credentials(
        snow_instance: String,
        credentials: CredentialType,
    ) -> GlideRecordConfig {
        GlideRecordConfig {
            credentials,
            snow_instance,
        }
    }

    pub fn snow_instance(&self) -> &str {
        &self.snow_instance
    }

    pub fn set_snow_instance(&mut self, snow_instance: String) {
        self.snow_instance = snow_instance;
    }

    pub fn set_credentials(&mut self, credentials: CredentialType) {
        self.credentials = credentials;
    }

    pub fn credentials(&self) -> &CredentialType {
        &self.credentials
    }
}
