/*
pub mod factland_api {
    include!(concat!(env!("OUT_DIR"), "/factland_api.rs"));
}
*/
use candid::{CandidType, Deserialize};
use ic_cdk::export::candid::candid_method;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;

type PrincipalString = String;

#[derive(Clone, Debug, Default, CandidType, Deserialize)]
struct Profile {
    updated_time_msecs: Option<u64>,
    username: Option<String>,
    password: Option<String>,
    email: Option<String>,
}

// Everything stored in State should be ephemeral in the sense that
// we can recover it case of a pre_upgrade failure.  For example,
// the assets can be rebuilt and the profiles are written through to
// stable storage.
#[derive(Clone, Debug, Default, CandidType, Deserialize)]
struct State {
    profiles: HashMap<PrincipalString, Profile>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct StableState {
    state: State,
    assets: ic_certified_assets::StableState,
}

#[ic_cdk_macros::update]
#[candid_method]
fn set_profile(profile: Profile) -> Profile {
    let user = ic_cdk::caller().to_text();
    STATE.with(|s| {
        if let Some(old_profile) = s.borrow().profiles.get(&user) {
            if old_profile.updated_time_msecs >= profile.updated_time_msecs {
                return old_profile.clone();
            }
        }
        *s.borrow_mut().profiles.get_mut(&user).unwrap() = profile.clone();
        profile
    })
}

#[ic_cdk_macros::update]
#[candid_method]
fn register(mut profile: Profile) -> Profile {
    let user = ic_cdk::caller().to_text();
    STATE.with(|s| {
        if !s.borrow().profiles.contains_key(&user) {
            if profile.updated_time_msecs == None {
                profile.updated_time_msecs = Some(ic_cdk::api::time() / 1000000);
            }
            if profile.username == None {
                profile.username = Some(user.clone());
            }
            if profile.password == None {
                let password = user.clone() + &ic_cdk::api::time().to_string();
                let mut hasher = Sha256::new();
                hasher.update(password);
                let password = hex::encode(hasher.finalize());
                profile.password = Some(password);
            }
            s.borrow_mut().profiles.insert(user.clone(), profile);
        }
        s.borrow().profiles[&user].clone()
    })
}

#[ic_cdk_macros::query]
#[candid_method]
fn login() -> Profile {
    let user = ic_cdk::caller().to_text();
    STATE.with(|s| {
        if !s.borrow().profiles.contains_key(&user) {
            ic_cdk::api::trap(&"User not found.");
        }
        s.borrow().profiles[&user].clone()
    })
}

#[ic_cdk_macros::init]
fn init() {
    ic_certified_assets::init();
}

#[ic_cdk_macros::pre_upgrade]
fn pre_upgrade() {
    let stable_state = STATE.with(|s| StableState {
        state: s.take(),
        assets: ic_certified_assets::pre_upgrade(),
    });
    ic_cdk::storage::stable_save((stable_state,)).expect("failed to save stable state");
}

#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    let (StableState { assets, state },): (StableState,) =
        ic_cdk::storage::stable_restore().expect("failed to restore stable state");
    ic_certified_assets::post_upgrade(assets);
    STATE.with(|s| {
        s.replace(state);
    });
}

ic_cdk::export::candid::export_service!();

#[ic_cdk_macros::query(name = "__get_candid_interface_tmp_hack")]
fn export_candid() -> String {
    __export_service()
}
