use std::error::Error;
/// A custom error type preventing panicking when unwraping
#[derive(Debug)]
pub enum TxError {
    NotEnoughAvailableFunds,
    AmountMandatory,
}
use std::fmt;
impl fmt::Display for TxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for TxError {}

use super::AccountState;
use super::TxId;
use std::marker::Sized;
pub trait Transaction: Sized {
    fn create(id: TxId, amount: Option<f64>) -> Result<Self, TxError>;
    /// Apply a transaction to a specific account
    /// This requires a mut ref to the account
    /// This operation can fail
    /// This consumes the transaction
    fn apply(self, acc: &mut AccountState) -> Result<(), TxError>;
}

#[derive(Debug)]
pub struct Deposit {
    id: TxId,
    amount: f64,
}

impl Transaction for Deposit {
    fn create(id: TxId, amount: Option<f64>) -> Result<Self, TxError> {
        let amount = amount.ok_or(TxError::AmountMandatory)?;

        Ok(Deposit { id, amount })
    }
    /// A deposit is a credit to the client's asset account,
    /// meaning it should increase the available and
    /// total funds of the client account
    fn apply(self, acc: &mut AccountState) -> Result<(), TxError> {
        acc.available += self.amount;
        acc.total += self.amount;

        // Register this tx into the account
        acc.register_deposit(self.id, self);

        Ok(())
    }
}
#[derive(Debug)]
pub struct Withdrawal {
    amount: f64,
}
impl Transaction for Withdrawal {
    fn create(_id: TxId, amount: Option<f64>) -> Result<Self, TxError> {
        let amount = amount.ok_or(TxError::AmountMandatory)?;

        Ok(Withdrawal { amount })
    }
    /// A deposit is a credit to the client's asset account,
    /// meaning it should increase the available and
    /// total funds of the client account
    fn apply(self, acc: &mut AccountState) -> Result<(), TxError> {
        if acc.available < self.amount {
            // not enough available funds,
            // the operation is canceled and the account
            // does not change
            Err(TxError::NotEnoughAvailableFunds)
        } else {
            acc.available -= self.amount;
            acc.total -= self.amount;

            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct Dispute {
    id: TxId,
}
impl Transaction for Dispute {
    fn create(id: TxId, _amount: Option<f64>) -> Result<Self, TxError> {
        Ok(Dispute { id })
    }
    /// A dispute represents a client's claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn't be reversed yet but the associated funds should be held. This means
    /// that the clients available funds should decrease by the amount disputed, their held funds should
    /// increase by the amount disputed, while their total funds should remain the same.
    fn apply(self, acc: &mut AccountState) -> Result<(), TxError> {
        // Get the depositf transaction from the account
        if let Some(tx) = acc.hist.get(&self.id) {
            acc.available -= tx.amount;
            acc.held += tx.amount;

            // Tell the account this transaction is disputed
            acc.register_dispute(self.id);
        }
        // otherwise it is ignored

        Ok(())
    }
}

pub struct Resolve {
    id: TxId,
}
impl Transaction for Resolve {
    fn create(id: TxId, _amount: Option<f64>) -> Result<Self, TxError> {
        Ok(Resolve { id })
    }
    /// A resolve represents a resolution to a dispute, releasing the associated held funds. Funds that
    /// were previously disputed are no longer disputed. This means that the clients held funds should
    /// decrease by the amount no longer disputed, their available funds should increase by the
    /// amount no longer disputed, and their total funds should remain the same.
    fn apply(self, acc: &mut AccountState) -> Result<(), TxError> {
        if acc.disputed_tx.contains(&self.id) {
            if let Some(tx) = acc.hist.get(&self.id) {
                acc.available += tx.amount;
                acc.held -= tx.amount;

                acc.unregister_dispute(self.id);
            }
        }
        // Ignored, error from the partner side

        Ok(())
    }
}

pub struct Chargeback {
    id: TxId,
}
impl Transaction for Chargeback {
    fn create(id: TxId, _amount: Option<f64>) -> Result<Self, TxError> {
        Ok(Chargeback { id })
    }
    /// A chargeback is the final state of a dispute and represents the client reversing a transaction.
    /// Funds that were held have now been withdrawn. This means that the clients held funds and
    /// total funds should decrease by the amount previously disputed. If a chargeback occurs the
    /// client's account should be immediately frozen.
    fn apply(self, acc: &mut AccountState) -> Result<(), TxError> {
        if acc.disputed_tx.contains(&self.id) {
            if let Some(tx) = acc.hist.get(&self.id) {
                // Client account frozen
                acc.locked = true;

                acc.total -= tx.amount;
                acc.held -= tx.amount;

                acc.unregister_dispute(self.id);
            }
        }

        Ok(())
    }
}
