use anyhow::{bail, Result};
use rust_decimal::Decimal;

use crate::transaction::{ClientId, Funds, TransactionId, Transactions};

pub struct ClientAccount {
    pub client: ClientId,
    pub available: Funds,
    pub held: Funds,
    pub total: Funds,
    pub locked: bool,
}

impl ClientAccount {
    pub fn new(client: ClientId) -> Self {
        Self {
            client,
            // Default funds = 0.0
            available: Decimal::new(0, 0),
            held: Decimal::new(0, 0),
            total: Decimal::new(0, 0),
            locked: false,
        }
    }
}

// NOTE:
// Deposit and Withdraw do not return Result, because even if they fail (if amount is None or
// because withdrawal was attempted with insufficient funds), I assume they still have to be kept
// in a transaction log.
//
// I'm not 100% certain about logging withdrawal in case of insufficient funds. What if there's a
// dispute for a failed withdrawal? It could be resolved and thus lose money. I'll keep this simple
// for now and log it anyway.
impl Transactions for ClientAccount {
    fn deposit(&mut self, tx: TransactionId, amount: Option<Funds>) {
        let Some(amount) = amount else { return }; // Deposit amount must be Some

        self.available += amount;
        self.total += amount;

        log::info!("ClientId: {client} - tx: {tx} - Deposited amount: {amount:.4} - new total: {total:.4}, new available: {available:.4}",
            client = self.client,
            total = self.total,
            available = self.available);
    }

    fn withdraw(&mut self, tx: TransactionId, amount: Option<Funds>) {
        let Some(amount) = amount else { return }; // Withdrawal amount must be Some

        if self.available < amount {
            log::warn!("ClientId: {client} - tx: {tx} - Failed to withdraw amount: {amount:.4} - Insufficient funds - total: {total:.4}, available: {available:.4}",
                client = self.client,
                total = self.total,
                available = self.available);

            return;
        }

        self.available -= amount;
        self.total -= amount;

        log::info!("ClientId: {client} - tx: {tx} - Withdrawed amount: {amount:.4} - new total: {total:.4}, new available: {available:.4}",
            client = self.client,
            total = self.total,
            available = self.available);
    }

    fn dispute(&mut self, tx: TransactionId, amount: Option<Funds>) -> Result<()> {
        let Some(amount) = amount else {
            bail!("ClientId: {client} - tx: {tx} - Failed to dispute transaction: Disputed transaction has an unspecified amount (is not Deposit or Withdrawal)", client = self.client);
        };

        self.available -= amount;
        self.held += amount;

        log::info!("ClientId: {client} - tx: {tx} - Disputed amount: {amount:.4} - new available: {available:.4}, new held: {held:.4}",
            client = self.client,
            available = self.available,
            held = self.held);

        Ok(())
    }

    fn resolve(&mut self, tx: TransactionId, amount: Option<Funds>) -> Result<()> {
        let Some(amount) = amount else {
            bail!("ClientId: {client} - tx: {tx} - Failed to resolve transaction: Resolved transaction has an unspecified amount (is not Deposit or Withdrawal)", client = self.client);
        };

        self.held -= amount;
        self.available += amount;

        log::info!("ClientId: {client} - tx: {tx} - Resolved amount: {amount:.4} - new available: {available:.4}, new held: {held:.4}",
            client = self.client,
            available = self.available,
            held = self.held);

        Ok(())
    }

    fn chargeback(&mut self, tx: TransactionId, amount: Option<Funds>) -> Result<()> {
        let Some(amount) = amount else {
            bail!("ClientId: {client} - tx: {tx} - Failed to resolve transaction: Resolved transaction has an unspecified amount (is not Deposit or Withdrawal)", client = self.client);
        };

        self.held -= amount;
        self.total -= amount;
        self.locked = true;

        log::info!("ClientId: {client} - tx: {tx} - Chargedback amount: {amount:.4} - new held: {held:.4}, new total: {total:.4} - Account has been locked.",
            client = self.client,
            held = self.held,
            total = self.total);

        Ok(())
    }
}
