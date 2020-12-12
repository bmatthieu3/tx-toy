use serde::Serialize;
use std::collections::{HashMap, HashSet};

use super::{ClientId, TxId};
use crate::transaction::Deposit;
#[derive(Serialize, Debug)]
pub struct AccountState {
    client: ClientId,
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,

    // History of deposit transactions referred
    // to this account
    // We use a HashMap and not a Vec
    // to efficiently find the transation given
    // its id.
    // a Vec<Tx> indexed by TxId would have led to
    // a very large array!
    #[serde(skip)]
    pub hist: HashMap<TxId, Deposit>,

    // Disputed transactions
    // By construction, disputed transactions are
    // registered in the hist above field
    #[serde(skip)]
    pub disputed_tx: HashSet<TxId>,
}

impl AccountState {
    pub fn new(client: ClientId) -> Self {
        AccountState {
            client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
            hist: HashMap::new(),
            disputed_tx: HashSet::new(),
        }
    }

    pub fn register_deposit(&mut self, id: TxId, tx: Deposit) {
        self.hist.insert(id, tx);
    }

    pub fn register_dispute(&mut self, id: TxId) {
        self.disputed_tx.insert(id);
    }

    pub fn unregister_dispute(&mut self, id: TxId) {
        // The dispute transaction has been handled (either Resolved or Chargebacked)
        // we can remove it from the set
        self.disputed_tx.remove(&id);
        // this dispute transaction was referring to a deposit
        // Does the deposit can be disputed again ???
        // I assume that no, so I remove the deposit from the history of deposits!
        // This will prevent the deposit hashmap to grow and grow...
        self.hist.remove(&id);
    }
}
