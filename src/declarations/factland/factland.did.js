export const idlFactory = ({ IDL }) => {
  const Profile = IDL.Record({
    'username' : IDL.Opt(IDL.Text),
    'password' : IDL.Opt(IDL.Text),
    'email' : IDL.Opt(IDL.Text),
    'updated_time_msecs' : IDL.Opt(IDL.Nat64),
  });
  return IDL.Service({
    'authorize' : IDL.Func([IDL.Principal], [], []),
    'authorize_controllers' : IDL.Func([], [], []),
    'backup' : IDL.Func([], [IDL.Vec(IDL.Tuple(IDL.Text, Profile))], ['query']),
    'deauthorize' : IDL.Func([IDL.Principal], [], []),
    'get_authorized' : IDL.Func([], [IDL.Vec(IDL.Principal)], ['query']),
    'login' : IDL.Func([], [Profile], ['query']),
    'register' : IDL.Func([Profile], [Profile], []),
    'restore' : IDL.Func([IDL.Vec(IDL.Tuple(IDL.Text, Profile))], [], []),
    'set_profile' : IDL.Func([Profile], [Profile], []),
    'stable_authorize' : IDL.Func([IDL.Principal], [], []),
    'stable_read' : IDL.Func(
        [IDL.Nat64, IDL.Nat64],
        [IDL.Vec(IDL.Nat8)],
        ['query'],
      ),
    'stable_size' : IDL.Func([], [IDL.Nat64], ['query']),
    'stable_write' : IDL.Func([IDL.Nat64, IDL.Vec(IDL.Nat8)], [], []),
  });
};
export const init = ({ IDL }) => { return []; };
