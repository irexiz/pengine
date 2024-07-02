use std::{
    collections::{btree_map::Entry, BTreeMap},
    io,
};

use anyhow::Result;

use crate::{
    client::ClientAccount,
    transaction::{
        ClientId, CsvOutput, Funds, Transaction, TransactionId, TransactionType, Transactions,
    },
};

pub struct ClientRepository {
    pub clients: BTreeMap<ClientId, ClientAccount>,
    pub transaction_log: BTreeMap<TransactionId, Transaction>,
}

impl ClientRepository {
    pub fn new() -> Self {
        Self {
            clients: BTreeMap::new(),
            transaction_log: BTreeMap::new(),
        }
    }

    pub fn output(self) -> Result<()> {
        let mut writer = csv::Writer::from_writer(io::stdout());

        for (client, account) in self.clients {
            writer.serialize(CsvOutput {
                client,
                available: account.available,
                held: account.held,
                total: account.total,
                locked: account.locked,
            })?;
        }

        Ok(())
    }

    pub fn process(&mut self, input: Transaction) -> Result<()> {
        let Transaction {
            typ,
            client,
            tx,
            amount,
        } = input;

        match self.clients.entry(client) {
            Entry::Occupied(mut o) => match typ {
                TransactionType::Deposit => {
                    o.get_mut().deposit(tx, amount);
                    self.log_transaction(typ, client, tx, amount);
                }
                TransactionType::Withdrawal => {
                    o.get_mut().withdraw(tx, amount);
                    self.log_transaction(typ, client, tx, amount);
                }
                TransactionType::Dispute => {
                    if let Some(transaction) = self.transaction_log.get_mut(&tx) {
                        // Only process transaction if it actually belonged to the client
                        if transaction.client == client {
                            o.get_mut().dispute(tx, transaction.amount)?;
                            transaction.typ = TransactionType::Dispute;
                        }
                    }
                }
                TransactionType::Resolve => {
                    if let Some(transaction) = self.transaction_log.get(&tx) {
                        // Only process transaction if its was disputed and it actually belonged to the client
                        if matches!(transaction.typ, TransactionType::Dispute)
                            && transaction.client == client
                        {
                            o.get_mut().resolve(tx, transaction.amount)?;
                        } else {
                            log::warn!("Skipping {typ:?} for client: {client}. Transaction for Resolve is not under Dispute or belongs to the wrong client");
                        }
                    }
                }
                TransactionType::Chargeback => {
                    if let Some(transaction) = self.transaction_log.get(&tx) {
                        // Only process transaction if its was disputed and it actually belonged to the client
                        if matches!(transaction.typ, TransactionType::Dispute)
                            && transaction.client == client
                        {
                            o.get_mut().chargeback(tx, transaction.amount)?;
                        } else {
                            log::warn!("Skipping {typ:?} for client: {client}. Transaction for Resolve is not under Dispute or belongs to the wrong client");
                        }
                    }
                }
            },
            // Client not previously present. Create account first - I assume there is no point in
            // trying to process a withdrawal/dispute/resolve/chargeback if there was no account
            // present in the first place, so I choose to only create an account on first deposit.
            Entry::Vacant(v) => match typ {
                TransactionType::Deposit => {
                    let mut client = ClientAccount::new(client);
                    client.deposit(tx, amount);
                    v.insert(client);
                }
                _ => log::warn!(
                    "Skipping {typ:?} for client: {client}. First transaction must be a Deposit."
                ),
            },
        }

        Ok(())
    }

    // Keep transaction log for Dispute/Resolve/Chargeback lookup purposes
    fn log_transaction(
        &mut self,
        typ: TransactionType,
        client: ClientId,
        tx: TransactionId,
        amount: Option<Funds>,
    ) {
        let transaction = Transaction {
            typ,
            client,
            tx,
            amount,
        };
        self.transaction_log.insert(tx, transaction);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deposit() {
        let mut client = ClientAccount {
            client: 1,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        };

        client.deposit(1, Some(10.0));

        assert_eq!(client.available, 10.0);
        assert_eq!(client.total, 10.0);
    }

    #[test]
    fn withdraw() {
        let mut client = ClientAccount {
            client: 1,
            available: 10.0,
            held: 0.0,
            total: 10.0,
            locked: false,
        };

        client.withdraw(1, Some(5.0));

        assert_eq!(client.available, 5.0);
        assert_eq!(client.total, 5.0);
    }

    #[test]
    fn withdraw_insufficient_funds() {
        let mut client = ClientAccount {
            client: 1,
            available: 1.0,
            held: 0.0,
            total: 1.0,
            locked: false,
        };

        client.withdraw(1, Some(5.0));

        assert_eq!(client.available, 1.0);
        assert_eq!(client.total, 1.0);
    }

    #[test]
    fn dispute_withdrawal() {
        let mut repo = ClientRepository::new();

        let transactions = vec![
            Transaction {
                typ: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(10.0),
            },
            Transaction {
                typ: TransactionType::Withdrawal,
                client: 1,
                tx: 2,
                amount: Some(3.0),
            },
            Transaction {
                typ: TransactionType::Dispute,
                client: 1,
                tx: 2,
                amount: None,
            },
        ];

        for transaction in transactions {
            let _ = repo.process(transaction);
        }

        let client = repo.clients.get(&1).unwrap();

        assert_eq!(client.total, 7.0);
        assert_eq!(client.available, 4.0);
        assert_eq!(client.held, 3.0);
    }

    #[test]
    fn resolve_withdrawal() {
        let mut repo = ClientRepository::new();

        let transactions = vec![
            Transaction {
                typ: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(10.0),
            },
            Transaction {
                typ: TransactionType::Withdrawal,
                client: 1,
                tx: 2,
                amount: Some(3.0),
            },
            Transaction {
                typ: TransactionType::Dispute,
                client: 1,
                tx: 2,
                amount: None,
            },
            Transaction {
                typ: TransactionType::Resolve,
                client: 1,
                tx: 2,
                amount: None,
            },
        ];

        for transaction in transactions {
            let _ = repo.process(transaction);
        }

        let client = repo.clients.get(&1).unwrap();

        assert_eq!(client.total, 7.0);
        assert_eq!(client.available, 7.0);
        assert_eq!(client.held, 0.0);
    }

    #[test]
    fn chargeback_withdrawal() {
        let mut repo = ClientRepository::new();

        let transactions = vec![
            Transaction {
                typ: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(10.0),
            },
            Transaction {
                typ: TransactionType::Withdrawal,
                client: 1,
                tx: 2,
                amount: Some(3.0),
            },
            Transaction {
                typ: TransactionType::Dispute,
                client: 1,
                tx: 2,
                amount: None,
            },
            Transaction {
                typ: TransactionType::Chargeback,
                client: 1,
                tx: 2,
                amount: None,
            },
        ];

        for transaction in transactions {
            let _ = repo.process(transaction);
        }

        let client = repo.clients.get(&1).unwrap();

        assert_eq!(client.total, 4.0);
        assert_eq!(client.available, 4.0);
        assert_eq!(client.held, 0.0);
        assert!(client.locked);
    }
}
