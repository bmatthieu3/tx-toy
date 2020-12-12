# Readme

## Correctness

I wrote some basics unit tests which you can find in the test directory.

```bash
cargo test
```

I did not used the type system to ensure correctness but that would be a
good idea (like a finite state machine sort of thing). For example a transaction type Dispute can
be converted to either type Resolved or Chargeback.
One point on the type chosen for the amount. I used a double but it is not the ideal
solution because the precision of doubles (and floats) drops for very big values.
For a float, passed ~1000 it is difficult to store 4 decimals digits.
For double it is something like 1e11 which is quite big.

## Safety and Robustness

There is **no unwrap** in the code, which reduces the chance to panic.
Instead there is a specific **TxError** type **implementing** the **std::error::Error**
trait. Methods return Result.
If an error occurs, the error is propagated to the main, it is printed and then
the program exits with a 1 status code.
**No** usage of **unsafe** code.

## Efficiency

**Serde** and **csv** crates are used which ensures some efficiency.
Transactions are processed as soon as they are received.
When transactions are all processed, the accounts can be serialized.

## Maintainability

The code is divided in three files (modularity).
A trait **Transaction** allows to extend the model and define new transaction
types more easily.
Results and custom error managing are used.
There are unit tests ensuring that new updates will not break the existent.
