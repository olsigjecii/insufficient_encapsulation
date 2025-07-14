use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

// --- Data Structures ---

/// Represents a simple bank account.
/// In the 'vulnerable' module, its fields are public, breaking encapsulation.
/// In the 'secure' module, its fields are private, enforcing encapsulation.
mod vulnerable_account {
    use serde::Serialize;
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize)]
    pub struct BankAccount {
        pub account_number: Uuid,
        pub balance: i32,
    }

    impl BankAccount {
        pub fn new(initial_balance: i32) -> Self {
            Self {
                account_number: Uuid::new_v4(),
                balance: initial_balance,
            }
        }
    }
}

mod secure_account {
    use serde::Serialize;
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize)]
    pub struct BankAccount {
        pub account_number: Uuid, // account_number can be public
        balance: i32,             // balance is private
    }

    impl BankAccount {
        pub fn new(initial_balance: i32) -> Self {
            Self {
                account_number: Uuid::new_v4(),
                balance: initial_balance,
            }
        }

        // Public getter for the balance to inspect it safely.
        pub fn balance(&self) -> i32 {
            self.balance
        }

        /// Securely deposits money.
        pub fn deposit(&mut self, amount: i32) {
            if amount > 0 {
                self.balance += amount;
            }
        }

        /// Securely withdraws money, checking for sufficient funds.
        /// This is our validation check that was bypassed in the vulnerable example.
        pub fn withdraw(&mut self, amount: i32) -> Result<(), &'static str> {
            if amount <= 0 {
                return Err("Withdrawal amount must be positive.");
            }
            if self.balance >= amount {
                self.balance -= amount;
                Ok(())
            } else {
                Err("Insufficient funds.")
            }
        }
    }
}

/// A struct to hold the shared application state.
/// We use two separate HashMaps to clearly distinguish between the
/// vulnerable and secure data models in this demonstration.
struct AppState {
    vulnerable_accounts: Mutex<HashMap<Uuid, vulnerable_account::BankAccount>>,
    secure_accounts: Mutex<HashMap<Uuid, secure_account::BankAccount>>,
}

#[derive(Deserialize)]
struct CreateAccountRequest {
    initial_balance: i32,
}

#[derive(Deserialize)]
struct TransferRequest {
    from_account: Uuid,
    to_account: Uuid,
    amount: i32,
}

// --- API Handlers ---

/// Creates a new bank account in both vulnerable and secure stores for demonstration.
async fn create_account(
    data: web::Data<AppState>,
    req: web::Json<CreateAccountRequest>,
) -> impl Responder {
    let mut vuln_accounts = data.vulnerable_accounts.lock().unwrap();
    let mut sec_accounts = data.secure_accounts.lock().unwrap();

    let vuln_account = vulnerable_account::BankAccount::new(req.initial_balance);
    let sec_account = secure_account::BankAccount::new(req.initial_balance);

    // To ensure both accounts have the same ID for easy comparison
    let new_id = vuln_account.account_number;
    let mut sec_account_mut = sec_account;
    sec_account_mut.account_number = new_id;

    vuln_accounts.insert(new_id, vuln_account.clone());
    sec_accounts.insert(new_id, sec_account_mut);

    HttpResponse::Ok().json(&vuln_account)
}

/// Retrieves an account's details (uses the secure model for display).
async fn get_account(data: web::Data<AppState>, path: web::Path<Uuid>) -> impl Responder {
    let account_id = path.into_inner();
    let sec_accounts = data.secure_accounts.lock().unwrap();

    match sec_accounts.get(&account_id) {
        Some(account) => HttpResponse::Ok().json(account),
        None => HttpResponse::NotFound().body("Account not found"),
    }
}

/// VULNERABLE transfer endpoint.
async fn vulnerable_transfer(
    data: web::Data<AppState>,
    req: web::Json<TransferRequest>,
) -> impl Responder {
    let mut accounts = data.vulnerable_accounts.lock().unwrap();

    // Direct access to fields, bypassing any logic or checks.
    let from_balance = accounts.get_mut(&req.from_account).map(|a| &mut a.balance);
    if let Some(balance) = from_balance {
        *balance -= req.amount; // No check for sufficient funds!
    } else {
        return HttpResponse::NotFound().body("Sender account not found");
    }

    let to_balance = accounts.get_mut(&req.to_account).map(|a| &mut a.balance);
    if let Some(balance) = to_balance {
        *balance += req.amount;
    } else {
        // NOTE: In a real scenario, this would require a transaction rollback.
        // Here, the sender's money is just gone.
        return HttpResponse::NotFound().body("Receiver account not found");
    }

    HttpResponse::Ok().body("Vulnerable transfer processed.")
}

/// SECURE transfer endpoint.
async fn secure_transfer(
    data: web::Data<AppState>,
    req: web::Json<TransferRequest>,
) -> impl Responder {
    let mut accounts = data.secure_accounts.lock().unwrap();

    // Edge case: A transfer to the same account is invalid.
    if req.from_account == req.to_account {
        return HttpResponse::BadRequest().body("Sender and receiver accounts cannot be the same.");
    }

    // Take ownership of the 'from' account by removing it from the map.
    // Now the HashMap is no longer borrowed, and we can work with it again.
    let mut from_account = match accounts.remove(&req.from_account) {
        Some(account) => account,
        None => return HttpResponse::NotFound().body("Sender account not found."),
    };

    // Now that 'from_account' is separate, we can safely take 'to_account'.
    let mut to_account = match accounts.remove(&req.to_account) {
        Some(account) => account,
        None => {
            // IMPORTANT: If the 'to_account' doesn't exist, we must put the 'from_account'
            // back into the map to cancel the transaction.
            accounts.insert(from_account.account_number, from_account);
            return HttpResponse::NotFound().body("Receiver account not found.");
        }
    };

    // --- Perform the validated operation ---
    // We now have full ownership of both accounts and can safely modify them.
    if let Err(e) = from_account.withdraw(req.amount) {
        // If the withdrawal fails, put both accounts back unchanged to abort the transaction.
        accounts.insert(from_account.account_number, from_account);
        accounts.insert(to_account.account_number, to_account);
        return HttpResponse::BadRequest().body(e); // e.g., "Insufficient funds."
    }

    // If withdrawal was successful, proceed with the deposit.
    to_account.deposit(req.amount);

    // --- Commit the transaction ---
    // The operation was successful, so put the modified accounts back into the map.
    accounts.insert(from_account.account_number, from_account);
    accounts.insert(to_account.account_number, to_account);

    HttpResponse::Ok().body("Secure transfer successful.")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize shared state
    let app_state = web::Data::new(AppState {
        vulnerable_accounts: Mutex::new(HashMap::new()),
        secure_accounts: Mutex::new(HashMap::new()),
    });

    println!("🚀 Server starting at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/accounts", web::post().to(create_account))
            .route("/accounts/{id}", web::get().to(get_account))
            // --- Vulnerable and Secure Paths ---
            .service(
                web::scope("/vulnerable").route("/transfer", web::post().to(vulnerable_transfer)),
            )
            .service(web::scope("/secure").route("/transfer", web::post().to(secure_transfer)))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
