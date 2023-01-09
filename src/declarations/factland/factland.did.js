export const idlFactory = ({ IDL }) => {
  const Profile = IDL.Record({
    'username' : IDL.Opt(IDL.Text),
    'password' : IDL.Opt(IDL.Text),
    'email' : IDL.Opt(IDL.Text),
    'updated_time_msecs' : IDL.Opt(IDL.Nat64),
  });
  return IDL.Service({
    'authorize' : IDL.Func([IDL.Principal], [], []),
    'backup' : IDL.Func([], [IDL.Vec(IDL.Tuple(IDL.Text, Profile))], ['query']),
    'deauthorize' : IDL.Func([IDL.Principal], [], []),
    'get_authorized' : IDL.Func([], [IDL.Vec(IDL.Principal)], ['query']),
    'login' : IDL.Func([], [Profile], ['query']),
    'register' : IDL.Func([Profile], [Profile], []),
    'restore' : IDL.Func([IDL.Vec(IDL.Tuple(IDL.Text, Profile))], [], []),
    'set_profile' : IDL.Func([Profile], [Profile], []),
  });
};
export const init = ({ IDL }) => { return []; };
