use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub type ClientId = u16;
pub type TransactionId = u32;
pub type Funds = Decimal;

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
    pub available: Funds,
    pub held: Funds,
    pub total: Funds,
    pub locked: bool,
}
