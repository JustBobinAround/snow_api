pub trait VarOrErr {
    fn var_or_err(var_name: &str) -> Result<String, GlideError>;
}

#[derive(Debug)]
pub enum GlideError {
    NoApiCredentialsFound,
    NoSnowInstanceFound,
    FailedToFetchBatch,
    FailedToDeserializeBatch,
    FailedToParseTableName,
    FailedToSerializeJson,
    RecordNotFound,
}
