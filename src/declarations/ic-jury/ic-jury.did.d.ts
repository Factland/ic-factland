import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export interface Profile {
  'username' : [] | [string],
  'password' : [] | [string],
  'email' : [] | [string],
  'updated_time_msecs' : [] | [bigint],
}
export interface _SERVICE {
  'login' : ActorMethod<[], Profile>,
  'register' : ActorMethod<[Profile], Profile>,
  'set_profile' : ActorMethod<[Profile], Profile>,
}
