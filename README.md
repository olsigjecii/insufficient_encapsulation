
# Lesson: Insufficient Encapsulation in Rust

Welcome to this lesson on Insufficient Encapsulation. We will explore how failing to properly protect data within its logical boundary can lead to serious bugs and security flaws. Using Rust and the actix-web framework, we will demonstrate how Rust's type system and privacy rules provide strong, compile-time protection against this issue.

📖 Concepts
Insufficient Encapsulation occurs when the internal data of an object or component can be directly modified by outside code, bypassing the intended methods (like setters or business logic functions) that are designed to interact with that data. This can lead to an invalid state, corruption of data, and circumvention of security controls.

In our demonstration, we model a bank transfer. The vulnerable code allows a transfer function to directly subtract from an account's balance, completely ignoring the check that ensures the account has sufficient funds. This allows the account's balance to become negative.

The mitigated code makes the balance field private. This forces any external code to use the public withdraw() and deposit() methods. The withdraw() method contains the critical check, preventing the balance from ever dropping below the withdrawal amount.

🛠️ Application Setup
Follow these steps to get the demo application running on your local machine.

Prerequisites
Install the Rust toolchain.

Instructions
Save the Code:

Save the first code block as src/main.rs.

Save the second code block as Cargo.toml.

Create the directory structure:

Bash

.
├── Cargo.toml
└── src
    └── main.rs
Build the Project:
Open your terminal in the project's root directory and run the build command.

Bash

cargo build
Run the Application:
Start the actix-web server.

Bash

cargo run
You should see the output: 🚀 Server starting at http://127.0.0.1:8080.

🔬 Demonstration
We will now interact with the running application to see the vulnerability and the fix in action. We recommend using two separate terminal windows for these curl commands.

Step 1: Create Two Bank Accounts
First, let's create two accounts. Account A will have a balance of 100, and Account B will have a balance of 50.

Execute the following commands. Note the account_number returned by each request—you will need them for the next steps.

Bash

# Create Account A with a balance of 100
curl -X POST -H "Content-Type: application/json" -d '{"initial_balance": 100}' http://127.0.0.1:8080/accounts

# Example Response (your ID will be different):
# {"account_number":"a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6","balance":100}

# Create Account B with a balance of 50
curl -X POST -H "Content-Type: application/json" -d '{"initial_balance": 50}' http://127.0.0.1:8080/accounts

# Example Response (your ID will be different):
# {"account_number":"f1e2d3c4-b5a6-f7e8-d9c0-b1a2f3e4d5c6","balance":50}
Let's say your account IDs are <ID_A> and <ID_B>.

Step 2: Demonstrate the Vulnerability
Now, we will attempt to transfer 200 from Account A (which only has 100) to Account B using the /vulnerable/transfer endpoint. This endpoint directly modifies the balance field without any checks.

Bash

# Replace <ID_A> and <ID_B> with your actual account numbers
curl -X POST -H "Content-Type: application/json" \
-d '{"from_account": "<ID_A>", "to_account": "<ID_B>", "amount": 200}' \
http://127.0.0.1:8080/vulnerable/transfer
The command will return a success message. Let's check the balance of Account A to see the result.

Bash

# Check the balance of Account A
curl http://127.0.0.1:8080/accounts/<ID_A>
Result:
You will see that Account A now has a negative balance, which should be impossible in a banking system.

JSON

{
  "account_number": "<ID_A>",
  "balance": -100
}
This demonstrates the danger of insufficient encapsulation. The business rule (non-negative balance) was bypassed because the balance field was exposed to direct manipulation.

Step 3: Demonstrate the Mitigation
Now, let's try the same operation with the secure endpoint. Please restart the application (Ctrl+C and cargo run) to reset the account balances to their initial state. Then, create two new accounts as you did in Step 1.

Attempt the same transfer of 200 from Account A to Account B using the /secure/transfer endpoint.

Bash

# Replace <ID_A> and <ID_B> with your new account numbers
curl -X POST -H "Content-Type: application/json" \
-d '{"from_account": "<ID_A>", "to_account": "<ID_B>", "amount": 200}' \
http://127.0.0.1:8080/secure/transfer
Result:
This time, the request fails with a clear error message from our withdraw method's internal check.

Insufficient funds.
Let's verify that the balances have not changed.

Bash

# Check Account A
curl http://127.0.0.1:8080/accounts/<ID_A>
# Expected output: {"account_number":"<ID_A>","balance":100}

# Check Account B
curl http://127.0.0.1:8080/accounts/<ID_B>
# Expected output: {"account_number":"<ID_B>","balance":50}
The balances are unchanged because the transaction was correctly aborted. The private balance field could only be modified through the withdraw() method, which enforced the application's rules. This is the power of proper encapsulation, a principle that Rust's privacy system helps enforce by default.