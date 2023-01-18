import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export interface Profile {
  'username' : [] | [string],
  'password' : [] | [string],
  'email' : [] | [string],
  'updated_time_msecs' : [] | [bigint],
}
export interface _SERVICE {
  'authorize' : ActorMethod<[Principal], undefined>,
  'authorize_controllers' : ActorMethod<[], undefined>,
  'backup' : ActorMethod<[], Array<[string, Profile]>>,
  'deauthorize' : ActorMethod<[Principal], undefined>,
  'get_authorized' : ActorMethod<[], Array<Principal>>,
  'login' : ActorMethod<[], Profile>,
  'register' : ActorMethod<[Profile], Profile>,
  'restore' : ActorMethod<[Array<[string, Profile]>], undefined>,
  'set_profile' : ActorMethod<[Profile], Profile>,
  'stable_authorize' : ActorMethod<[Principal], undefined>,
  'stable_read' : ActorMethod<[bigint, bigint], Uint8Array>,
  'stable_size' : ActorMethod<[], bigint>,
  'stable_write' : ActorMethod<[bigint, Uint8Array], undefined>,
}
