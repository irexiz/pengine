use anyhow::Result;
use serde::{Deserialize, Serialize, Serializer};

pub type ClientId = u16;
pub type TransactionId = u32;
pub type Funds = f32;

pub trait Transactions {
    fn deposit(&mut self, tx: TransactionId, amount: Option<Funds>);
    fn withdraw(&mut self, tx: TransactionId, amount: Option<Funds>);
    fn dispute(&mut self, tx: TransactionId, amount: Option<Funds>) -> Result<()>;
    fn resolve(&mut self, tx: TransactionId, amount: Option<Funds>) -> Result<()>;
    fn chargeback(&mut self, tx: TransactionId, amount: Option<Funds>) -> Result<()>;
}

#[derive(Debug, Eq, PartialEq, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Transaction {
    #[serde(rename = "type")]
    pub typ: TransactionType,
    pub client: ClientId,
    pub tx: TransactionId,
    #[serde(default, deserialize_with = "csv::invalid_option")]
    pub amount: Option<Funds>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub struct CsvOutput {
    pub client: ClientId,
    #[serde(serialize_with = "round_serialize")]
    pub available: Funds,
    #[serde(serialize_with = "round_serialize")]
    pub held: Funds,
    #[serde(serialize_with = "round_serialize")]
    pub total: Funds,
    pub locked: bool,
}

// Output Funds with 4 decimal point precision
fn round_serialize<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{x:.4}"))
}
