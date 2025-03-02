const idlFactory = ({ IDL }) => {
  const MetadataValue = IDL.Variant({
    'Nat' : IDL.Nat,
    'Int' : IDL.Int,
    'Text' : IDL.Text,
    'Blob' : IDL.Vec(IDL.Nat8)
  });
  return IDL.Service({
    'metadata' : IDL.Func(
      [],
      [IDL.Vec(IDL.Tuple(IDL.Text, MetadataValue))],
      ['query']
    )
  });
};

export { idlFactory };