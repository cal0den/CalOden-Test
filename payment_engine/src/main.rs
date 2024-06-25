use std::env;
use std::error::Error;
use csv::ReaderBuilder;
use std::fs::File;
use std::collections::HashMap;
use csv::Trim;

fn main() -> Result<(), Box<dyn Error>> {
    // Read the command line arguments.
    let input: Vec<String> = env::args().collect();

    // Create our hashmaps to store client and transaction details
    // clients hashmap: client_id -> (available, held, frozen)
    let mut clients: HashMap<String, (f64, f64, f64, bool)> = HashMap::new();
    // transaction_history hashmap: transaction_id -> (client_id, amount, disputed)
    let mut transaction_history: HashMap<String, (String, f64, bool)> = HashMap::new();

    // Check that we have the correct number of arguments
    if input.len() != 2 {
        eprintln!("Usage: cargo run -- <input.csv> > <output.csv>");
        std::process::exit(1);
    }

    // Open the CSV file.
    let input_file = File::open(&input[1])?;

    // Create a CSV reader
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All)
        .from_reader(input_file);

    // Iterate over lines in the CSV.
    for record in rdr.records() {
        let result = match record {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error reading CSV record: {}", e);
                // Handle the error (e.g., skip the record)
                continue;
            }
        };
        // Each transaction is a `Result<StringRecord, csv::Error>`
        let current_transaction = match Transaction::from_record(result) {
            Ok(transaction) => transaction,
            Err(e) => {
                eprintln!("Error processing transaction: {}", e);
                continue;
            }
        };
        let current_transaction_clone = current_transaction.clone();

        // Skip transactions for frozen accounts
        let current_client_lock_state = match clients.get(&current_transaction.client_id) {
            Some((_available, _held, _total, frozen)) => *frozen,
            None => false,
        };
        if current_client_lock_state {
            eprintln!("Skipping transaction for frozen account: {}", current_transaction.client_id);
            continue;
        }

        // Process the transaction based on its type
        match current_transaction.transaction_type.as_str() {
            // Deposits increase available balance and total balance
            "deposit" | "withdrawal"=> {
                let current_transaction_type = &current_transaction_clone.transaction_type.as_str();

                // Skip transactions if we already have a key for that transaction_id
                if transaction_history.contains_key(&current_transaction_clone.transaction_id) {
                    eprintln!("Skipping duplicate transaction: {}", current_transaction.transaction_id);
                    continue;
                }
                clients = current_transaction.deposit_or_withdrawal(clients, current_transaction_type);
                transaction_history.insert(current_transaction_clone.transaction_id, (current_transaction_clone.client_id, current_transaction_clone.amount, false));
            },
            // Disputes decrease available balance and increase held balance
            "dispute" => {
                let current_transaction_id = &current_transaction_clone.transaction_id;
                let current_client_id = &current_transaction_clone.client_id;

                // If the transaction is not found in the transaction history or
                // the client_id does not match the disputed transaction, skip it
                if !transaction_history.contains_key(current_transaction_id) {
                    eprintln!("Transaction ID {} not found in transaction history. Skipping", current_transaction_id);
                    continue;
                }
                else if transaction_history.get(current_transaction_id).unwrap().0 != *current_client_id {
                    eprintln!("Client ID, {}, given does not match the disputed transaction. Skipping", current_client_id);
                    continue;
                }

                // Store the amount from the disputed transaction
                let disputed_amount = transaction_history.get(current_transaction_id).unwrap().1.clone();
                // Process dispute transactions
                clients = current_transaction.dispute(clients, transaction_history.clone());

                // Mark the transaction as disputed for an existing transaction in the transaction_history hashmap
                transaction_history.insert((current_transaction_id).to_string(), (current_transaction_clone.client_id.clone(), disputed_amount, true));
            },
            // Resolves increase available balance and decrease held balance
            // Chargebacks decrease held balance and freeze the account
            "resolve" | "chargeback" => {
                let current_transaction_id = &current_transaction_clone.transaction_id;
                let current_transaction_type = &current_transaction_clone.transaction_type.as_str();

                // If the transaction is not found in the transaction history or
                // the transaction is not disputed, skip it
                if !transaction_history.contains_key(current_transaction_id) {
                    eprintln!("Transaction ID {} not found in transaction history. Skipping", current_transaction_id);
                    continue;
                }
                else if !transaction_history.get(current_transaction_id).unwrap().2 {
                    eprintln!("Skipping {} transaction for non-disputed transaction: {}", current_transaction.transaction_type, current_transaction_id);
                    continue;
                }

                // Process resolve or chargeback transactions. In either case, the transaction is marked as undisputed
                clients = current_transaction.resolve_or_chargeback(clients, transaction_history.clone(), current_transaction_type);
                transaction_history.insert((current_transaction_id).to_string(), (current_transaction_clone.client_id.clone(), current_transaction_clone.amount, false));
            }
            // All other transaction types are skipped
            _ => {
                eprintln!("Skipping unknown transaction type: {}", current_transaction.transaction_type);
            }
        }

    }

    println!("client, available, held, total, locked");
    for client in clients.iter() {
        println!("{}, {:.4}, {:.4},  {:.4}, {}", client.0, client.1.0, client.1.1, client.1.2, client.1.3);
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Transaction {
    transaction_type: String,
    client_id: String,
    transaction_id: String,
    amount: f64,
}

impl Transaction {
    fn from_record(record: csv::StringRecord) -> Result<Self, Box<dyn Error>> {
        if record.len() != 4 {
            return Err("Expected at least 4 fields in the CSV record".into());
        }

        let transaction_type = record.get(0).unwrap().to_string();
        let client_id = record.get(1).unwrap().to_string();
        let transaction_id = record.get(2).unwrap().to_string();

        // Parse the amount as a float, default to zero if it is empty
        let amount_str = record.get(3).unwrap().to_string();
        let amount = match amount_str.parse::<f64>() {
            Ok(parsed_amount) => parsed_amount,
            Err(_) => {if amount_str.is_empty() {
                        0.0
                    }
                       else {
                        return Err("Failed to parse amount as f64".into())
                    }
            },
        };
        // Check if the supplied IDs are valid integers, even though we use them
        // as strings in this implementation. This should also catch empty values.
        if client_id.parse::<u16>().is_err() {
            return Err(format!("Client ID '{}' is not a valid integer", client_id).into());
        }
        else if transaction_id.parse::<u32>().is_err() {
            return Err(format!("Transaction ID '{}' is not a valid integer", transaction_id).into());
        }

        Ok(Transaction {
            transaction_type,
            client_id,
            transaction_id,
            amount
        })
    }

    // Process deposit and withdrawal transactions
    fn deposit_or_withdrawal(self,
                             mut clients: HashMap<String, (f64, f64, f64, bool)>,
                             action: &str
    ) -> HashMap<String, (f64, f64, f64, bool)> {
        // Use Entry API to insert or update client's account
        let entry = clients.entry(self.client_id.clone());
        let (available, _held, total, _frozen) = entry.or_insert((0.0, 0.0, 0.0, false));

        if action == "deposit" {
            // Update balances based on transaction type
            *available += self.amount;
            *total += self.amount;
        } else if action == "withdrawal" {
            // Update balances based on transaction type
            *available -= self.amount;
            *total -= self.amount;
        }

        clients
    }

    // Process dispute transactions
    fn dispute(self,
               mut clients: HashMap<String, (f64, f64, f64, bool)>,
               transaction_history: HashMap<String, (String, f64, bool)>
        ) -> HashMap<String, (f64, f64, f64, bool)> {
        // Use Entry API to insert or update client's account
        let entry = clients.entry(self.client_id.clone());
        let (available, held, total, _frozen) = entry.or_insert((0.0, 0.0, 0.0, false));

        // Get the transaction details for the disputed transaction
        let (_client_id, amount, _disputed) = transaction_history.get(&self.transaction_id).unwrap().clone();

        // Update balances based on transaction type
        *available -= amount;
        *held += amount;
        *total = *available + *held;

        clients
    }

    // Process resolve and chargeback transactions
    fn resolve_or_chargeback(self,
                             mut clients: HashMap<String, (f64, f64, f64, bool)>,
                             transaction_history: HashMap<String, (String, f64, bool)>,
                             action: &str
    ) -> HashMap<String, (f64, f64, f64, bool)> {
        // Use Entry API to insert or update client's account
        let entry = clients.entry(self.client_id.clone());
        let (available, held, total, frozen) = entry.or_insert((0.0, 0.0, 0.0, false));
        // Get the transaction details for the disputed transaction
        let (client_id, amount, _disputed) = transaction_history.get(&self.transaction_id).unwrap().clone();

        // Check client_id matches the disputed transaction
        if client_id != self.client_id {
            eprintln!("Client ID does not match disputed transaction: {}", self.client_id);
            return clients;
        }
        else {
            // Update balances based on transaction type
            if action == "resolve" {
                *available += amount;
                *held -= amount;
            } else if action == "chargeback" {
                *held -= amount;
                *frozen = true;
            }

            *total = *available + *held;
        }
        clients
    }
}
