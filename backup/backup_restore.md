# Backup and Restore of Data on the Internet Computer

This article describes how to backup, restore and disaster recover stable memory data in Rust canister smart contracts on the Internet Computer.

The Internet Computer uses [Stable Memory](https://internetcomputer.org/docs/current/references/ic-interface-spec#system-api-stable-memory) to store data over upgrades to the canister smart contract.  For Rust, this memory can be managed by a library of data structures e.g. [stable-structures](https://github.com/dfinity/stable-structures). Access to this memory is entirely under control of the smart contract so any defect in that contract can result in damage to that data, and in the worst case a broken contract.  Debugging, maintenance and disaster recovery are all possible if we can backup, view, modify and restore stable memory.

## A dApp with Stable Structures

This example is taken from the decentralized fact checking dApp [Factland](https://factland.org) and the code is available on [github](https://github.com/Factland/ic-factland).  Consider this code fragment which uses stable-structures to store user profiles:

```rust
#[derive(Clone, Debug, Default, CandidType, Deserialize)]
struct Profile {
    updated_time_msecs: Option<u64>,
    username: Option<String>,
    password: Option<String>,
    email: Option<String>,
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static PROFILES: RefCell<StableBTreeMap<Memory, PrincipalStorable, Profile>> = RefCell::new(
        StableBTreeMap::init_with_sizes(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
          MAX_PROFILES_KEY_SIZE,
          MAX_PROFILES_VALUE_SIZE
          )
        );
}
```

Here we are defining a stable memory manager and a `BTreeMap` from `Principal` (wrapped in a Storable so that it can be stored in stable memory) to a user `Profile`.  Eventually this dApp will be controlled by a DAO via the [SNS](https://internetcomputer.org/docs/current/tokenomics/sns/sns-intro-tokens) but particularly while it is in active development we want to be able to backup and restore the profiles to enable a fast and safe development cycle.  Note that we are using `thread_local` and `RefCell` to tell Rust that we are operating in a single threaded environment.

## Application Level Backup and Restore

The first thing we can do is export the data at the application level:

```rust
#[ic_cdk_macros::query(guard = "is_authorized")]
#[candid_method]
fn backup(offset: u32, count: u32) -> Vec<(String, Profile)> {
  PROFILES.with(|p| {
      p.borrow()
      .iter()
      .skip(offset as usize)
      .take(count as usize)
      .map(|(k, p)| (k.0.to_text(), p))
      .collect()
      })
}
```

Then we can restore that data:

```rust
#[ic_cdk_macros::update(guard = "is_authorized")]
#[candid_method]
fn restore(profiles: Vec<(String, Profile)>) {
  PROFILES.with(|m| {
    let mut m = m.borrow_mut();
    for p in profiles {
      let principal = PrincipalStorable(Principal::from_text(p.0).unwrap());
      m.insert(principal, p.1).unwrap();
    }
  });
}
```

Of course we cannot allow just any user to download this data, so it must be protected by a guard:

```rust
fn is_authorized() -> Result<(), String> {
  AUTH.with(|a| {
    if a.borrow() .contains_key(&ic_cdk::caller().as_slice().to_vec()) {
      Ok(())
    } else {
      Err("You are not authorized".to_string())
    }
  }
}
```

The authorization data is also stored in stable memory:

```rust
#[derive(Clone, Debug, CandidType, Deserialize, FromPrimitive)]
enum Auth {
  Admin,
}

... within the thread_local! block
static AUTH: RefCell<StableBTreeMap<Memory, Blob, u32>> = RefCell::new(
  StableBTreeMap::init_with_sizes(
    MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))),
    MAX_AUTH_KEY_SIZE,
    4
    )
  );
```

Client code is available in the Factland repo for [backup](https://github.com/Factland/ic-factland/tree/main/backup/backup.js) and [restore](https://github.com/Factland/ic-factland/tree/main/backup/restore.js).  This code written in javascript and runs on nodejs.  It uses the deploying Principal to authorize an operator Principal which does the backkup, saving the data in JSON format, and restore.  Once the operator Principal is authorized and the client is initialized the core code just prints out the data in JSON format:

```javascript
let profiles = await actor.backup();
console.log(JSON.stringify(profiles));
```

The restore code is similarly straightforward:

```javascript
const profiles = JSON.parse(fs.readFileSync('./backup.dat'));
await actor.restore(profiles);
```

## Low level Backup and Restore

If for any reason the dApp fails catastrophically, the stable memory memory can still be backed up, viewed and modified offline, repaired and restored.  To support this we need to be able to get the size of the stable memory, read and write it:
  

```rust
#[ic_cdk_macros::query(guard = "is_stable_authorized")]
#[candid_method]
fn stable_size() -> u64 {
  ic_cdk::api::stable::stable64_size() * WASM_PAGE_SIZE
}

#[ic_cdk_macros::query(guard = "is_stable_authorized")]
#[candid_method]
fn stable_read(offset: u64, length: u64) -> Vec<u8> {
  let mut buffer = Vec::new();
  buffer.resize(length as usize, 0);
  ic_cdk::api::stable::stable64_read(offset, buffer.as_mut_slice());
  buffer
}

#[ic_cdk_macros::update(guard = "is_stable_authorized")]
#[candid_method]
fn stable_write(offset: u64, buffer: Vec<u8>) {
  let size = offset + buffer.len() as u64;
  let old_size = ic_cdk::api::stable::stable64_size() * WASM_PAGE_SIZE;
  if size > old_size {
    let old_pages = old_size / WASM_PAGE_SIZE;
    let pages = (size + (WASM_PAGE_SIZE - 1)) / WASM_PAGE_SIZE;
    ic_cdk::api::stable::stable64_grow(pages - old_pages).unwrap();
  }
  ic_cdk::api::stable::stable64_write(offset, buffer.as_slice());
}
```

Of course there is a chicken and egg problem with authorization.  The standard authorization for the dApp uses stable memory, so we need low level authorization stored in canister memory which will be ephemeral and will be lost when the canister is upgraded.  This is less convenient, but low level backup and restore is not expected to be used frequently:

```rust
  ... within the thread_local! block
static AUTH_STABLE: RefCell<HashSet<Principal>> = RefCell::new(HashSet::<Principal>::new());

fn is_stable_authorized() -> Result<(), String> {
  AUTH_STABLE.with(|a| {
    if a.borrow().contains(&ic_cdk::caller()) {
      Ok(())
    } else {
      Err("You are not stable authorized".to_string())
    }
  })
}
```

In order to read and write backed up images of stable stable memory, we can compile the dApp and run it locally using conditional compilation directives to differentiate the environment:

```rust
thread_local! {
#[cfg(not(target_arch = "wasm32"))]
static MEMORY_MANAGER: RefCell<MemoryManager<FileMemory>> =
    RefCell::new(MemoryManager::init(FileMemory::new(File::open("backup/stable_memory.dat").unwrap())));
#[cfg(target_arch = "wasm32")]
static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
  RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
...                         
}
```

Then in a `main()` function which will only be run locally we can access the stable memory image:

```rust
#[cfg(not(target_arch = "wasm32"))]
fn main() {
  let principals = get_authorized();
  println!("authorized principals: {}", principals.len());
  for p in principals {
    println!("  {}", p.to_text());
  }
}
```

The low level backup and restore javascript is also available in the [Factland](https://factland.org) [github repo](https://github.com/Factland/ic-factland). Backup and restore operates on one megabyte blocks.  In order to make this relatively fast the backup uses queries and the restore can compare a modified image to an original image, restoring only the blocks which have been changed.
