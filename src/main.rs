use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_cdk::export::candid::candid_method;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fmt::Debug;

type Memory = VirtualMemory<DefaultMemoryImpl>;
type Blob = Vec<u8>;

const MAX_PROFILES_KEY_SIZE: u32 = 64;
const MAX_PROFILES_VALUE_SIZE: u32 = 256;

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
    static PROFILES: RefCell<StableBTreeMap<Memory, Blob, Blob>> = RefCell::new(
        StableBTreeMap::init_with_sizes(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
            MAX_PROFILES_KEY_SIZE,
            MAX_PROFILES_VALUE_SIZE
            )
        );
}

#[ic_cdk_macros::update]
#[candid_method]
fn set_profile(profile: Profile) -> Profile {
    let user = ic_cdk::caller().to_text().as_bytes().to_vec();
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
    let user_text = ic_cdk::caller().to_text();
    let user = user_text.as_bytes().to_vec();
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
            .insert(user.clone(), Encode!(&profile).unwrap())
            .unwrap();
        profile
    })
}

#[ic_cdk_macros::query]
#[candid_method]
fn login() -> Profile {
    let user = ic_cdk::caller().to_text().as_bytes().to_vec();
    PROFILES.with(|p| {
        if !p.borrow().contains_key(&user) {
            ic_cdk::api::trap(&"User not found.");
        }
        Decode!(&p.borrow().get(&user).unwrap(), Profile).unwrap()
    })
}

#[ic_cdk_macros::query]
#[candid_method]
fn backup() -> Vec<(String, Profile)> {
    PROFILES.with(|p| {
        p.borrow()
            .iter()
            .map(|(k, p)| {
                (
                    String::from_utf8_lossy(&k).to_string(),
                    Decode!(&p, Profile).unwrap(),
                )
            })
            .collect()
    })
}

#[ic_cdk_macros::update]
#[candid_method]
fn restore(profiles: Vec<(String, Profile)>) {
    let user_text = ic_cdk::caller().to_text();
    let user = user_text.as_bytes().to_vec();
    ic_cdk::eprintln!("{} {}", user_text, user.len());
    PROFILES.with(|m| {
        let mut m = m.borrow_mut();
        for p in profiles {
            m.insert(p.0.as_bytes().to_vec(), Encode!(&p.1).unwrap())
                .unwrap();
        }
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
