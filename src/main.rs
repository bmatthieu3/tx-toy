use std::error::Error;
use std::io;
use std::process;

use serde::Deserialize;

type ClientId = u16;
type TxId = u32;

mod account_state;
use account_state::AccountState;

mod transaction;
use transaction::{Chargeback, Deposit, Dispute, Resolve, Transaction, Withdrawal};
#[derive(Debug, Deserialize)]
struct Record {
    #[serde(rename = "type")]
    _type: Type,
    #[serde(rename = "client")]
    client: ClientId,
    #[serde(rename = "tx")]
    tx: TxId,
    #[serde(rename = "amount")]
    // A f32 does not always guarantee 4 digits past
    // the decimal but ~7 digits all in all i.e. past
    // 1000 we only have ~3 more digit for decimals...
    // A f64 can store up to 16 digits of precision i.e.
    // past 1e11 it is not possible to store 4 more decimals.
    // One could use maybe two unsigned integers, u64 storing what
    // is left the comma and a u16 for what is right
    amount: Option<f64>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Type {
    Deposit,
    Dispute,
    Withdraw,
    Resolve,
    Chargeback,
}

use std::fs::File;
fn process_tx(in_filename: &str) -> Result<(), Box<dyn Error>> {
    let accounts = rd_from_stdin(in_filename)?;
    wtr_to_stdout(accounts)?;

    Ok(())
}

fn rd_from_stdin(in_filename: &str) -> Result<Vec<Option<AccountState>>, Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        //.flexible(true)
        .from_reader(File::open(in_filename)?);

    // We know there is not much than 65536 clients
    // hence it is ok to already preallocate the accounts in a vec
    let mut accounts: Vec<Option<AccountState>> = (0..u16::MAX).map(|_| None).collect();
    for result in rdr.deserialize() {
        // Deserialization of the record
        let Record {
            _type,
            client,
            tx,
            amount,
        } = result?;
        // Get the good account
        let acc = if let Some(acc) = &mut accounts[client as usize - 1] {
            acc
        } else {
            // No client found, we create one account for him
            let acc = AccountState::new(client);
            accounts[client as usize - 1] = Some(acc);
            accounts[client as usize - 1].as_mut().unwrap()
        };

        // If the account is locked, we process skip the
        // application of further transactions
        if acc.locked {
            continue;
        }

        // Create the transaction and apply it to the account
        match _type {
            Type::Deposit => {
                // create the tx
                let tx = Deposit::create(tx, amount)?;
                // and apply it directly to the account
                tx.apply(acc)?;
            }
            Type::Withdraw => {
                // create the tx
                let tx = Withdrawal::create(tx, amount)?;
                // and apply it directly to the account
                tx.apply(acc)?;
            }
            Type::Dispute => {
                // create the tx
                let tx = Dispute::create(tx, amount)?;
                // and apply it directly to the account
                tx.apply(acc)?;
            }
            Type::Resolve => {
                // create the tx
                let tx = Resolve::create(tx, amount)?;
                // and apply it directly to the account
                tx.apply(acc)?;
            }
            Type::Chargeback => {
                // create the tx
                let tx = Chargeback::create(tx, amount)?;
                // and apply it directly to the account
                tx.apply(acc)?;
            }
        }
    }

    Ok(accounts)
}

fn wtr_to_stdout(accounts: Vec<Option<AccountState>>) -> Result<(), Box<dyn Error>> {
    // When all the transactions have been applied
    // we output the account statuses
    let mut wtr = csv::Writer::from_writer(io::stdout());
    for account in accounts.into_iter() {
        if let Some(acc) = account {
            wtr.serialize(acc)?;
        }
    }
    wtr.flush()?;
    Ok(())
}

// To read CLI arguments, e.g.
// the name of the input CSV file
use std::env;
fn main() {
    let args: Vec<String> = env::args().collect();

    // Index 0 is the name of the program
    if args.len() == 1 {
        println!("No input filename given");
        process::exit(1);
    } else {
        if args.len() >= 3 {
            println!("Multiple arguments given.\n Only the first one will be considered.");
        }

        let in_filename = &args[1];

        if let Err(err) = process_tx(&in_filename) {
            println!("error occured: {}", err);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rd_from_stdin;
    #[test]
    fn deposit() {
        let accounts = rd_from_stdin("test/basic.csv").unwrap();
        for acc in accounts.into_iter() {
            if let Some(acc) = acc {
                assert_eq!(acc.held + acc.available, acc.total);
            }
        }
    }

    #[test]
    fn deposit_and_withdrawal() {
        assert!(rd_from_stdin("test/not_available_funds.csv").is_err());
    }

    #[test]
    fn sums_up_to_zero() {
        assert_eq!(
            rd_from_stdin("test/sums_up_to_zero.csv").unwrap()[0]
                .as_ref()
                .unwrap()
                .total,
            0.0
        );
    }

    #[test]
    fn two_clients() {
        let accounts = rd_from_stdin("test/two_clients.csv").unwrap();
        let acc1 = accounts[0].as_ref().unwrap();
        let acc2 = accounts[1].as_ref().unwrap();

        assert_eq!(acc1.total, 9.0);
        assert_eq!(acc1.held, 0.0);
        assert_eq!(acc1.available, 9.0);

        assert_eq!(acc2.total, 10.0);
        assert_eq!(acc2.held, 4.0);
        assert_eq!(acc2.available, 6.0);
    }

    #[test]
    fn dispute() {
        let accounts = rd_from_stdin("test/dispute.csv").unwrap();
        let acc = accounts[0].as_ref().unwrap();
        assert_eq!(acc.total, 0.0);
        assert_eq!(acc.held, 4.0);
        assert_eq!(acc.available, -4.0);
    }

    #[test]
    fn dispute_invalid_ignored() {
        let accounts = rd_from_stdin("test/dispute_invalid.csv").unwrap();
        let acc = accounts[0].as_ref().unwrap();

        assert_eq!(acc.total, 0.0);
        assert_eq!(acc.held, 0.0);
        assert_eq!(acc.available, 0.0);
    }

    #[test]
    fn dispute_resolved() {
        let accounts = rd_from_stdin("test/dispute_resolved.csv").unwrap();
        let acc = accounts[0].as_ref().unwrap();

        assert_eq!(acc.total, 4.0);
        assert_eq!(acc.held, 0.0);
        assert_eq!(acc.available, 4.0);
    }

    #[test]
    fn chargeback() {
        let accounts = rd_from_stdin("test/chargeback.csv").unwrap();
        let acc = accounts[0].as_ref().unwrap();

        assert_eq!(acc.total, -3.0);
        assert_eq!(acc.held, 0.0);
        assert_eq!(acc.available, -3.0);
        assert!(acc.locked);
    }
}
