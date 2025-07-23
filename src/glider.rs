use crate::{
    auth::{CredentialType, GlideRecordConfig},
    errors::GlideError,
};
use reqwest::blocking::Client;
use serde::Deserializer;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::VecDeque;

//TODO: better error handling for things like insertion/update errors
pub trait Glideable: Serialize + DeserializeOwned {
    fn sys_id(&self) -> &String;
    fn update(&self, gr: &GlideRecord<Self>) -> Option<Self>;
    fn insert(&self, gr: &GlideRecord<Self>) -> Option<Self>;
    fn delete(&self, gr: &GlideRecord<Self>) -> Option<Self>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlideResponse<T> {
    result: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlideResponseBatch<T> {
    result: VecDeque<T>,
}
impl<T> GlideResponseBatch<T> {
    pub fn new() -> GlideResponseBatch<T> {
        GlideResponseBatch {
            result: VecDeque::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GlideReference {
    table: String,
    sys_id: String,
}

impl Default for GlideReference {
    fn default() -> Self {
        GlideReference {
            table: String::new(),
            sys_id: String::new(),
        }
    }
}

#[derive(Deserialize)]
struct PreprocessedGlideRef {
    link: String,
}

impl PreprocessedGlideRef {
    pub fn into_glide_ref(mut pre: Self) -> GlideReference {
        let mut sys_id = String::new();
        let mut table = String::new();

        while let Some(c) = pre.link.pop() {
            if c == '/' {
                break;
            }
            sys_id.push(c);
        }

        while let Some(c) = pre.link.pop() {
            if c == '/' {
                break;
            }
            table.push(c);
        }

        let sys_id = sys_id.chars().rev().collect();
        let table = table.chars().rev().collect();

        GlideReference { table, sys_id }
    }
}

impl<'de> serde::Deserialize<'de> for GlideReference {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let glide_ref = PreprocessedGlideRef::deserialize(d)
            .ok()
            .map(|temp| PreprocessedGlideRef::into_glide_ref(temp))
            .unwrap_or_default();

        Ok(glide_ref)
    }
}

impl GlideReference {
    pub fn as_glide_record<T: Serialize + DeserializeOwned>(
        &self,
    ) -> Result<GlideRecord<T>, GlideError> {
        let mut gr = GlideRecord::new(&self.table)?;
        gr.add_encoded_query(&format!("sys_id={}", self.sys_id));
        gr.query()?;
        Ok(gr)
    }

    pub fn as_item<T: Serialize + DeserializeOwned>(&self) -> Result<T, GlideError> {
        let mut gr = GlideRecord::new(&self.table)?;
        match gr.get(&self.sys_id) {
            Some(item) => Ok(item),
            None => Err(GlideError::RecordNotFound),
        }
    }

    pub fn as_glide_record_with_config<T: Serialize + DeserializeOwned>(
        &self,
        config: GlideRecordConfig,
    ) -> Result<GlideRecord<T>, GlideError> {
        let gr = GlideRecord::new_with_configuration(&self.table, config);
        Ok(gr)
    }
}

pub struct GlideRecord<T: Serialize + DeserializeOwned> {
    table_name: String,
    encoded_query: String,
    query_limit: u32,
    offset: u32,
    len: Option<u32>,
    config: GlideRecordConfig,
    query_lock: bool,
    batch_size: u32,
    current_batch: GlideResponseBatch<T>,
    client: Client,
}

impl<T: Serialize + DeserializeOwned> GlideRecord<T> {
    pub fn new(table_name: &str) -> Result<GlideRecord<T>, GlideError> {
        let config = GlideRecordConfig::new_from_env_vars()?;

        Ok(GlideRecord {
            table_name: table_name.to_string(),
            encoded_query: String::new(),
            query_limit: 10000,
            offset: 0,
            len: None,
            config,
            query_lock: false,
            batch_size: 10000,
            current_batch: GlideResponseBatch::new(),
            client: reqwest::blocking::Client::new(),
        })
    }

    pub fn new_with_configuration(table_name: &str, config: GlideRecordConfig) -> GlideRecord<T> {
        GlideRecord {
            table_name: table_name.to_string(),
            encoded_query: String::new(),
            query_limit: 10000,
            offset: 0,
            len: None,
            config,
            query_lock: false,
            batch_size: 10000,
            current_batch: GlideResponseBatch::new(),
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn add_encoded_query(&mut self, query: &str) {
        if self.encoded_query.len() == 0 {
            self.encoded_query.push_str(query);
        } else {
            self.encoded_query.push('^');
            self.encoded_query.push_str(query);
        }
        self.query_lock = false;
    }

    pub fn get_encoded_query(&self) -> &str {
        &self.encoded_query
    }

    pub fn set_limit(&mut self, limit: u32) {
        self.query_limit = limit;
        if self.batch_size > self.query_limit {
            self.batch_size = self.query_limit;
        }
    }

    pub fn set_batch_size(&mut self, batch_size: u32) {
        self.batch_size = batch_size;
        if self.batch_size > self.query_limit {
            self.batch_size = self.query_limit;
        }
    }

    fn table_api_link(&self) -> String {
        format!(
            "https://{}/api/now/table/{}?sysparm_query={}&sysparm_limit={}&sysparm_offset={}",
            self.config.snow_instance(),
            self.table_name,
            self.encoded_query,
            self.batch_size,
            self.offset
        )
    }

    fn table_api_record_insert_link(&self) -> String {
        format!(
            "https://{}/api/now/table/{}",
            self.config.snow_instance(),
            self.table_name,
        )
    }

    fn table_api_record_link(&self, sys_id: &str) -> String {
        format!(
            "https://{}/api/now/table/{}/{}",
            self.config.snow_instance(),
            self.table_name,
            sys_id
        )
    }

    fn next_batch(&mut self) -> Result<(), GlideError> {
        let no_more_batches = self.len.is_some_and(|len| len < self.offset);

        if no_more_batches {
            return Ok(());
        }

        let request = self
            .client
            .get(self.table_api_link())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");

        let request = match &self.config.credentials() {
            CredentialType::Basic { user_name, passwd } => {
                request.basic_auth(user_name, Some(passwd))
            }
            CredentialType::Token(token) => request.bearer_auth(token),
        };

        let response = match request.send() {
            Ok(response) => response,
            Err(_) => return Err(GlideError::FailedToFetchBatch),
        };

        self.len = response
            .headers()
            .get("x-total-count")
            .map(|len| len.to_str().ok())
            .unwrap_or(None)
            .map(|len| u32::from_str_radix(len, 10).ok())
            .unwrap_or(None);

        self.current_batch = match response.json() {
            Ok(json) => json,
            Err(_) => {
                return Err(GlideError::FailedToDeserializeBatch);
            }
        };
        Ok(())
    }

    pub fn query(&mut self) -> Result<(), GlideError> {
        self.query_lock = true;
        self.next_batch()?;
        Ok(())
    }

    pub fn next(&mut self) -> Option<T> {
        if !self.query_lock || self.offset >= self.query_limit {
            return None;
        }

        let next_item = match self.current_batch.result.pop_front() {
            Some(item) => Some(item),
            None => {
                let _ = self.next_batch();
                self.current_batch.result.pop_front()
            }
        };

        if next_item.is_some() {
            self.offset += 1;
        }

        next_item
    }

    pub fn get(&mut self, sys_id: &str) -> Option<T> {
        let request = self
            .client
            .get(self.table_api_record_link(sys_id))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");

        let request = match &self.config.credentials() {
            CredentialType::Basic { user_name, passwd } => {
                request.basic_auth(user_name, Some(passwd))
            }
            CredentialType::Token(token) => request.bearer_auth(token),
        };

        request
            .send()
            .ok()
            .map(|response| response.json().ok())
            .map(|glide_response| glide_response.map(|gr: GlideResponse<T>| gr.result))
            .unwrap_or(None)
    }

    pub fn insert(&self, to_insert: &T) -> Option<T> {
        let json = match serde_json::to_string(&to_insert) {
            Ok(json) => json,
            Err(_) => return None,
        };

        let request = self
            .client
            .post(self.table_api_record_insert_link())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(json);

        let request = match &self.config.credentials() {
            CredentialType::Basic { user_name, passwd } => {
                request.basic_auth(user_name, Some(passwd))
            }
            CredentialType::Token(token) => request.bearer_auth(token),
        };

        request
            .send()
            .ok()
            .map(|response| response.json().ok())
            .map(|glide_response| glide_response.map(|gr: GlideResponse<T>| gr.result))
            .unwrap_or(None)
    }

    pub fn update(&self, to_update: &T, sys_id: &str) -> Option<T> {
        let json = match serde_json::to_string(&to_update) {
            Ok(json) => json,
            Err(_) => return None,
        };

        let request = self
            .client
            .put(self.table_api_record_link(sys_id))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(json);

        let request = match &self.config.credentials() {
            CredentialType::Basic { user_name, passwd } => {
                request.basic_auth(user_name, Some(passwd))
            }
            CredentialType::Token(token) => request.bearer_auth(token),
        };

        request
            .send()
            .ok()
            .map(|response| response.json().ok())
            .map(|glide_response| glide_response.map(|gr: GlideResponse<T>| gr.result))
            .unwrap_or(None)
    }

    pub fn delete(&self, sys_id: &str) -> Option<T> {
        let request = self
            .client
            .put(self.table_api_record_link(sys_id))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");

        let request = match &self.config.credentials() {
            CredentialType::Basic { user_name, passwd } => {
                request.basic_auth(user_name, Some(passwd))
            }
            CredentialType::Token(token) => request.bearer_auth(token),
        };

        request
            .send()
            .ok()
            .map(|response| response.json().ok())
            .map(|glide_response| glide_response.map(|gr: GlideResponse<T>| gr.result))
            .unwrap_or(None)
    }
}
