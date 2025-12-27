## Why Merkle-Search Tree
In distributed system, "shared" contract is often inevitable. 
If we apply this in replication, MST can be used to sync with great robusteness to capture the difference.
Applying to a rather service level application such as micro services, it will capture the drift between services. 



## Requirement - The "Shared Contract"
For MST to work between two different processes, they don't need to have the same schema, but they must agree on a Sync Contract.
They need to agree on two things:
- The Key: A unique ID present in both systems (e.g., order_id or correlation_id).
- The Value (The Hash): A specific subset of fields that must match.


## "Anti-Corruption" Mapper
Imagine your Order Service has 50 columns. Your Billing Service has 20 columns. You cannot hash the "Row." You must hash a "Mapped Object."

Example Logic:
- Order Service: Reads `db_orders` -> Extracts {id, amount, currency, status} -> Hashes it.
- Billing Service: Reads `db_invoices` -> Extracts {order_ref_id, total, currency, payment_state} -> Hashes it.

You must normalize the data before hashing. If Order Service stores "USD" and Billing stores "usd", the hashes will differ, and MST will report a false mismatch.


## The "Canonicalization" Trap
This is the primary reason MST implementations fail in many micro services.
JSON is unordered. 
- Service A hashes: {"id": 1, "amt": 100} -> Hash 0xABC 
- Service B hashes: {"amt": 100, "id": 1} -> Hash 0xXYZ

MST reports a mismatch every single time, even if data is identical.
This means that you must use Canonical Serialization (e.g., standardizing field order) before hashing (Proto buf or Avro do this natively.)



## Why MST over event streaming?
Events miss things. 
- Broker can go down
- Consumer crashes and commits the offsets when it shouldn't
- A "re-drive" script runs and updates the DB without emitting an event.

MST looks at the actual database state, not the event stream. 