# Factland backend

This canister smart contract contains the Factland user data backend.

## Usage

User profiles contain:

```
type Profile = record {
  updated_time_msecs: opt nat64;
  username: opt text;
  password: opt text;
  email: opt text;
};
```

The contract presents the API:

```
service factland: {
  register:  (Profile) -> (Profile);
  login:  () -> (Profile) query;
  set_profile: (Profile) -> (Profile);
  backup: () -> (vec record { text; Profile }) query;
  restore: (vec record { text; Profile }) -> ();
  //
  // Manage the set of Principals allowed to backup/restore.
  //
  authorize: (principal) -> ();
  deauthorize: (principal) -> ();
  get_authorized: () -> (vec principal) query;
}
```

## Backup and Restore

The canister uses stable memory to store all data, so it is not necessary to backup and restore the data under normal operation.  Nevertheless, the data can be backed up and restored e.g. to support arbitrary schema changes. The principal doing the backup and restore must first be authorized by the principal which installed the canister smart contract.  Sample code to backup and restore the data is the `./backup` directory.

The installation principal is assumed to be available as the default in `dfx`.  The backup and restore identity is assumed to be `factland`.  The canister id of the canister smart contract is hard coded in the scripts.  These can be changed in the code.

### Backup

```
node backup.js > backup.dat
```

### Restore

```
node restore.js
```

## Development

### Depenedencies

* node, npm
* rustup, cargo, rustc with wasm

### Setup

* (cd backup; npm i)

### Build

* make build

### Test

* dfx start --background
* dfx deploy
* make test
