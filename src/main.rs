use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_cdk::api::management_canister::main::{canister_status, CanisterIdRecord};
use ic_cdk::export::candid::candid_method;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fmt::Debug;
#[macro_use]
extern crate num_derive;

type Memory = VirtualMemory<DefaultMemoryImpl>;
type Blob = Vec<u8>;

const MAX_PROFILES_KEY_SIZE: u32 = 32;
const MAX_PROFILES_VALUE_SIZE: u32 = 256;
const MAX_AUTH_KEY_SIZE: u32 = 32;
const WASM_PAGE_SIZE: u64 = 65536;

#[derive(Clone, Debug, Default, CandidType, Deserialize)]
struct Profile {
    updated_time_msecs: Option<u64>,
    username: Option<String>,
    password: Option<String>,
    email: Option<String>,
}

#[derive(Clone, Debug, CandidType, Deserialize, FromPrimitive)]
enum Auth {
    Admin,
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static PROFILES: RefCell<StableBTreeMap<Memory, Blob, Blob>> = RefCell::new(
        StableBTreeMap::init_with_sizes(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
            MAX_PROFILES_KEY_SIZE,
            MAX_PROFILES_VALUE_SIZE
            )
        );
    static AUTH: RefCell<StableBTreeMap<Memory, Blob, u32>> = RefCell::new(
        StableBTreeMap::init_with_sizes(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))),
            MAX_AUTH_KEY_SIZE,
            4
            )
        );
}

#[ic_cdk_macros::update]
#[candid_method]
fn set_profile(profile: Profile) -> Profile {
    let user = ic_cdk::caller().as_slice().to_vec();
    PROFILES.with(|p| {
        if let Some(old_profile) = p.borrow().get(&user) {
            let old_profile = Decode!(&old_profile, Profile).unwrap();
            if old_profile.updated_time_msecs >= profile.updated_time_msecs {
                return old_profile.clone();
            }
        }
        p.borrow_mut()
            .insert(user, Encode!(&profile).unwrap())
            .unwrap();
        profile
    })
}

#[ic_cdk_macros::update]
#[candid_method]
async fn register(mut profile: Profile) -> Profile {
    let user = ic_cdk::caller().as_slice().to_vec();
    let user_text = ic_cdk::caller().to_text();
    PROFILES.with(|p| {
        if !p.borrow().contains_key(&user) {
            if profile.updated_time_msecs == None {
                profile.updated_time_msecs = Some(ic_cdk::api::time() / 1000000);
            }
            if profile.username == None {
                profile.username = Some(user_text.clone());
            }
        } else {
            ic_cdk::api::trap(&"User already registered.");
        }
    });
    if profile.password == None {
        let raw_rand: Vec<u8> =
            match ic_cdk::call(Principal::management_canister(), "raw_rand", ()).await {
                Ok((res,)) => res,
                Err((_, err)) => ic_cdk::trap(&format!("failed to get rand: {}", err)),
            };
        profile.password = Some(hex::encode(Sha256::digest(raw_rand)));
    }
    PROFILES.with(|p| {
        p.borrow_mut()
            .insert(user, Encode!(&profile).unwrap())
            .unwrap();
        profile
    })
}

#[ic_cdk_macros::query]
#[candid_method]
fn login() -> Profile {
    let user = ic_cdk::caller().as_slice().to_vec();
    PROFILES.with(|p| {
        if !p.borrow().contains_key(&user) {
            ic_cdk::api::trap(&"User not found.");
        }
        Decode!(&p.borrow().get(&user).unwrap(), Profile).unwrap()
    })
}

#[ic_cdk_macros::query(guard = "is_authorized")]
#[candid_method]
fn backup() -> Vec<(String, Profile)> {
    PROFILES.with(|p| {
        p.borrow()
            .iter()
            .map(|(k, p)| {
                (
                    Principal::from_slice(&k).to_text(),
                    Decode!(&p, Profile).unwrap(),
                )
            })
            .collect()
    })
}

#[ic_cdk_macros::update(guard = "is_authorized")]
#[candid_method]
fn restore(profiles: Vec<(String, Profile)>) {
    PROFILES.with(|m| {
        let mut m = m.borrow_mut();
        for p in profiles {
            let principal = Principal::from_text(p.0).unwrap();
            m.insert(principal.as_slice().to_vec(), Encode!(&p.1).unwrap())
                .unwrap();
        }
    });
}

// This is an update so that it can read the controllers.
#[ic_cdk_macros::update]
#[candid_method]
async fn stable_size() -> u64 {
    if !is_controller().await {
        ic_cdk::api::trap(&"Not controller.");
    }
    ic_cdk::api::stable::stable64_size() * WASM_PAGE_SIZE
}

// This is an update so that it can read the controllers.
#[ic_cdk_macros::update]
#[candid_method]
async fn stable_read(offset: u64, length: u64) -> Vec<u8> {
    if !is_controller().await {
        ic_cdk::api::trap(&"Not controller.");
    }
    let mut buffer = Vec::new();
    buffer.resize(length as usize, 0);
    ic_cdk::api::stable::stable64_read(offset, buffer.as_mut_slice());
    buffer
}

#[ic_cdk_macros::update]
#[candid_method]
async fn stable_write(offset: u64, buffer: Vec<u8>) {
    if !is_controller().await {
        ic_cdk::api::trap(&"Not controller.");
    }
    let size = offset + buffer.len() as u64;
    let old_size = ic_cdk::api::stable::stable64_size() * WASM_PAGE_SIZE;
    if size > old_size {
        let old_pages = old_size / WASM_PAGE_SIZE;
        let pages = (size + (WASM_PAGE_SIZE - 1)) / WASM_PAGE_SIZE;
        ic_cdk::api::stable::stable64_grow(pages - old_pages).unwrap();
    }
    ic_cdk::api::stable::stable64_write(offset, buffer.as_slice());
}

#[ic_cdk_macros::query]
#[candid_method]
fn get_authorized() -> Vec<Principal> {
    let mut authorized = Vec::new();
    AUTH.with(|a| {
        for (k, _v) in a.borrow().iter() {
            authorized.push(Principal::from_slice(&k));
        }
    });
    authorized
}

#[ic_cdk_macros::update(guard = "is_authorized")]
#[candid_method]
fn authorize(principal: Principal) {
    authorize_principal(&principal);
}

#[ic_cdk_macros::update(guard = "is_authorized")]
#[candid_method]
fn deauthorize(principal: Principal) {
    AUTH.with(|a| {
        a.borrow_mut()
            .remove(&principal.as_slice().to_vec())
            .unwrap();
    });
}

#[ic_cdk_macros::init]
fn canister_init() {
    authorize_principal(&ic_cdk::caller());
}

fn is_authorized() -> Result<(), String> {
    AUTH.with(|a| {
        if a.borrow()
            .contains_key(&ic_cdk::caller().as_slice().to_vec())
        {
            Ok(())
        } else {
            Err("You are not authorized".to_string())
        }
    })
}

async fn is_controller() -> bool {
    let caller = ic_cdk::caller();
    let status = canister_status(CanisterIdRecord {
        canister_id: ic_cdk::api::id(),
    })
    .await
    .unwrap();
    status.0.settings.controllers.contains(&caller)
}

fn authorize_principal(principal: &Principal) {
    let value = Auth::Admin;
    AUTH.with(|a| {
        a.borrow_mut()
            .insert(principal.as_slice().to_vec(), value as u32)
            .unwrap();
    });
}

ic_cdk::export::candid::export_service!();

#[ic_cdk_macros::query(name = "__get_candid_interface_tmp_hack")]
fn export_candid() -> String {
    __export_service()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("{}", export_candid());
}

#[cfg(target_arch = "wasm32")]
fn main() {}
