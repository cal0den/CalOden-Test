# Transaction Processing Tool

This tool handles **deposit**, **withdrawal**, **dispute**, **resolve**, and **chargeback** transactions. It reads a CSV file one line at a time by using `csv::ReaderBuilder`, which allows it to avoid pushing the entire file to memory. It then handles each transaction based on the transaction type given in the CSV file. If the transaction type isn't recognized, the IDs aren't valid integers, or the amount given is a string, then that transaction will be skipped, and the relevant error will be printed to the console.

It is assumed that clients' accounts can become overdrawn, represented by a negative balance.

## Usage

To use this tool, run the following command from the command line, inserting the relevant names for CSV files where appropriate:

```sh
cargo run -- <input.csv> > <output.csv>
```

## Testing

Testing for this is basic but robust. I have created a series of csv files which test:

1. deposits and withdrawals (creating clients and testing that transactions still apply if the transaction IDs are supplied out of order but still chronologically correct for each client).
2. deposits and disputes (where disputes for missing transaction IDs shown to be rejected and where disputes with incorrect client IDs are rejected)
3. deposits, disputes and resolves
4. deposits, disputes and chargebacks
5. All of the above in one, larger csv file

The expected output of these tests, not necessarily in the order given below, is the following:

**1\)**
**In CSV file:**
```
client, available, held, total, locked
1, -4.5000, 0.0000,  -4.5000, false
2, -3.0000, 0.0000,  -3.0000, false
3, -1.5000, 0.0000,  -1.5000, false
4, 0.0000, 0.0000,  0.0000, false
5, 1.5000, 0.0000,  1.5000, false
```
**2\)**
**In console:**
```sh
Client ID, 1, given does not match the disputed transaction. Skipping
Client ID, 3, given does not match the disputed transaction. Skipping
```
**In CSV file:**
```
client, available, held, total, locked
1, 0.0000, 2.0000,  2.0000, false
2, 0.0000, 2.0000,  2.0000, false
```

**3\)**
**In CSV file:**
```
client, available, held, total, locked
1, 2.0000, 0.0000,  2.0000, false
2, 0.0000, 2.0000,  2.0000, false
```

**4\)**
**In console:**
```sh
Skipping chargeback transaction for non-disputed transaction: 2
```
**In CSV file:**
```
client, available, held, total, locked
1, 0.0000, 0.0000,  0.0000, true
2, 2.0000, 0.0000,  2.0000, false
```
**5\)**
**In console:**
```sh
Skipping chargeback transaction for non-disputed transaction: 6
Skipping duplicate transaction: 6
```
**In CSV file:**
```
client, available, held, total, locked
1, 5.0000, 0.0000,  5.0000, false
2, 7.0000, 0.0000,  7.0000, false
3, 0.0000, 0.0000,  0.0000, true
4, -8.0000, 0.0000,  -8.0000, false
```

## Final thoughts

This tool could have been made to be more efficient if it was asynchronous and multi-threaded, using libraries like **tokio**. If this was a server tool taking many concurrent requests I would expect one or more threads to be polling for requests and then storing them somewhere while multiple other threads read the stored data and process the transactions. Storing the requests in memory is necessary as otherwise extremely high volumes of concurrent requests would lead to requests being dropped.